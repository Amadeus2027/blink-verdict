//! One killable child per task. Keys travel only through inherited private pipes.
use super::*;
use std::{
    io::{BufRead, BufReader, Write},
    os::windows::{io::AsRawHandle, process::CommandExt},
    process::{Child, Command, Stdio},
    sync::mpsc,
};
use windows::Win32::System::JobObjects::*;

#[derive(serde::Serialize, serde::Deserialize)]
struct Request {
    config: Config,
    key: String,
    preview: Option<String>,
    rect: [i32; 4],
    hide: bool,
    operation: i64,
}

struct Job(HANDLE);
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

pub(super) struct Worker {
    child: Child,
    _job: Job,
    receiver: mpsc::Receiver<WorkerEvent>,
    deadline: Instant,
}
impl Worker {
    pub fn start(
        config: Config,
        key: String,
        preview: Option<String>,
        rect: RECT,
        hide: bool,
        operation: i64,
    ) -> Result<Self, String> {
        let budget = if hide || preview.is_some() {
            12
        } else {
            config.timeout_seconds + 20
        };
        let request = Request {
            config,
            key,
            preview,
            rect: [rect.left, rect.top, rect.right, rect.bottom],
            hide,
            operation,
        };
        let path = std::env::current_exe().map_err(|_| "无法定位任务程序".to_string())?;
        let mut command = Command::new(path);
        command.arg("--decision-worker");
        Self::spawn(command, Some(request), Duration::from_secs(budget.into()))
    }

    fn spawn(
        mut command: Command,
        request: Option<Request>,
        budget: Duration,
    ) -> Result<Self, String> {
        unsafe {
            let job =
                Job(CreateJobObjectW(None, None).map_err(|_| "无法创建任务资源组".to_string())?);
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
            .map_err(|_| "无法设置任务回收策略".to_string())?;
            let mut child = command
                .creation_flags(CREATE_NO_WINDOW.0)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|_| "无法启动决策任务".to_string())?;
            if AssignProcessToJobObject(job.0, HANDLE(child.as_raw_handle())).is_err() {
                let _ = child.kill();
                return Err("无法管理决策任务，请检查系统限制".into());
            }
            let mut input = child.stdin.take().unwrap();
            let output = child.stdout.take().unwrap();
            let (sender, receiver) = mpsc::channel();
            std::thread::Builder::new()
                .name("decision-pipe".into())
                .spawn(move || {
                    if let Some(request) = request {
                        if serde_json::to_writer(&mut input, &request).is_err() {
                            let _ =
                                sender.send(WorkerEvent::Finished(Err("任务输入传输失败".into())));
                            return;
                        }
                    }
                    drop(input);
                    for line in BufReader::new(output).lines() {
                        let event = line
                            .ok()
                            .and_then(|line| serde_json::from_str::<WorkerEvent>(&line).ok());
                        match event {
                            Some(event) => {
                                if sender.send(event).is_err() {
                                    break;
                                }
                            }
                            None => {
                                let _ = sender.send(WorkerEvent::Finished(Err(
                                    "任务输出异常，请重试".into(),
                                )));
                                break;
                            }
                        }
                    }
                })
                .map_err(|_| "无法创建任务通信线程".to_string())?;
            Ok(Self {
                child,
                _job: job,
                receiver,
                deadline: Instant::now() + budget,
            })
        }
    }

    pub fn poll(&mut self) -> Option<WorkerEvent> {
        // After a timeout the owner drops this worker, its job and its receiver.
        // No late result can enter another request's channel.
        if Instant::now() >= self.deadline {
            let _ = self.child.kill();
            return Some(WorkerEvent::Finished(Err(
                "本次判断超时，任务已回收；可以重新按键或调整连接等待时间".into(),
            )));
        }
        match self.receiver.try_recv() {
            Ok(event) => Some(event),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(WorkerEvent::Finished(Err(
                "决策任务意外结束，已恢复；可以重新按键".into(),
            ))),
        }
    }
}

fn emit(event: &WorkerEvent) -> Result<(), String> {
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(&mut output, event).map_err(|_| "任务结果传输失败".to_string())?;
    output
        .write_all(b"\n")
        .and_then(|_| output.flush())
        .map_err(|_| "任务结果传输失败".to_string())
}

pub fn run_if_requested() -> bool {
    if !std::env::args_os().any(|arg| arg == "--decision-worker") {
        return false;
    }
    let request: Request = match serde_json::from_reader(std::io::stdin().lock()) {
        Ok(r) => r,
        Err(_) => return true,
    };
    unsafe {
        let initialized = RoInitialize(RO_INIT_MULTITHREADED).is_ok();
        let result = execute(request);
        if initialized {
            RoUninitialize();
        }
        let _ = emit(&WorkerEvent::Finished(result));
    }
    true
}

unsafe fn execute(request: Request) -> Result<(String, bool, String), String> {
    let Request {
        config,
        key,
        preview,
        rect: [left, top, right, bottom],
        hide,
        operation,
    } = request;
    let rect = RECT {
        left,
        top,
        right,
        bottom,
    };
    if hide {
        let bridge = gamebar::Bridge::connect(&config.gamebar_package_family, operation)?;
        bridge.send(&config, "hide", "", rect)?;
        return Ok((String::new(), true, String::new()));
    }

    let bridge = if config.gamebar_package_family.is_empty() {
        None
    } else {
        Some(gamebar::Bridge::connect(
            &config.gamebar_package_family,
            operation,
        )?)
    };
    if let Some(bridge) = &bridge {
        bridge.send(&config, "hide", "", rect)?;
    }
    if let Some(text) = preview {
        if let Some(bridge) = &bridge {
            bridge.send(&config, "show", &text, rect)?;
        }
        return Ok((text, bridge.is_some(), "样式预览（未调用模型）".into()));
    }
    let observation = capture::observe(rect, config.vision)?;
    if config.mode == DisplayMode::Widget {
        if let Some(bridge) = &bridge {
            bridge.send(&config, "processing", "正在思考…", rect)?;
        } else {
            emit(&WorkerEvent::Captured)?;
        }
    }
    let verdict = match provider::decide(&config, &key, observation) {
        Ok(verdict) => verdict,
        Err(error) => {
            if let Some(bridge) = &bridge {
                let _ = bridge.send(&config, "hide", "", rect);
            }
            return Err(error);
        }
    };
    let mut details = String::new();
    if let Some(confidence) = verdict.confidence {
        details.push_str(&format!(
            "\n模型置信度：{:.1}%（由服务提供，并非正确率保证）",
            confidence * 100.0
        ));
    }
    for (i, p) in verdict.probabilities.iter().enumerate() {
        if let Some(p) = p {
            let label = config
                .options
                .get(i)
                .map(String::as_str)
                .unwrap_or("无法判断");
            details.push_str(&format!("\n{label}：{:.1}%", p * 100.0));
        }
    }
    let text = match verdict.verdict {
        Verdict::Choice(index) => config.options[index].clone(),
        Verdict::Uncertain => "无法判断".into(),
    };
    if let Some(bridge) = &bridge {
        bridge.send(&config, "show", &text, rect)?;
    }
    Ok((text, bridge.is_some(), details))
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn drive(app: &mut App) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.busy && Instant::now() < deadline {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message != WM_QUIT {
                    DispatchMessageW(&msg);
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!app.busy, "host never recovered");
        assert!(app.worker.is_none());
    }

    #[test]
    #[ignore = "targeted Windows regression: actual child timeout/exit/retry and shortcut guard"]
    fn runtime_regression_worker_recovery_and_shortcut() {
        unsafe {
            let mut fixture = TestApp::new();
            let app = &mut fixture.0;
            let mut command = Command::new("powershell.exe");
            command.args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 60",
            ]);
            let task = Worker::spawn(command, None, Duration::from_millis(150)).unwrap();
            let process = OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                false,
                task.child.id(),
            )
            .unwrap();
            app.worker = Some(task);
            app.busy = true;
            assert_ne!(SetTimer(app.host, WORKER_TIMER_ID, 50, None), 0);
            drive(app);
            assert!(app.status.contains("超时"));
            assert_eq!(WaitForSingleObject(process, 2000), WAIT_OBJECT_0);
            let _ = CloseHandle(process);

            let mut exit = Command::new("cmd.exe");
            exit.args(["/C", "exit 7"]);
            app.worker = Some(Worker::spawn(exit, None, Duration::from_secs(3)).unwrap());
            app.busy = true;
            SetTimer(app.host, WORKER_TIMER_ID, 50, None);
            drive(app);
            assert!(app.status.contains("意外结束"));

            let blocker = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                None,
                WINDOW_STYLE(0),
                0,
                0,
                1,
                1,
                None,
                None,
                GetModuleHandleW(None).unwrap(),
                None,
            )
            .unwrap();
            let (modifiers, key) = hotkey::parse("Ctrl+Alt+F24").unwrap();
            RegisterHotKey(blocker, 42, modifiers, key).unwrap();
            assert!(app.register_hotkey("Ctrl+Alt+F24").is_err());
            app.trigger(); // No registered shortcut must not start a request.
            assert!(app.worker.is_none());
            let _ = UnregisterHotKey(blocker, 42);
            let _ = DestroyWindow(blocker);

            // A new real worker succeeds after the hung and failed tasks. No model call.
            let executable = std::env::var_os("BLINK_VERDICT_TEST_EXE")
                .expect("build release and set BLINK_VERDICT_TEST_EXE");
            let mut command = Command::new(executable);
            command.arg("--decision-worker");
            let request = Request {
                config: app.config.clone(),
                key: String::new(),
                preview: Some("YES".into()),
                rect: [0, 0, 1920, 1080],
                hide: false,
                operation: 0,
            };
            app.worker =
                Some(Worker::spawn(command, Some(request), Duration::from_secs(5)).unwrap());
            app.busy = true;
            SetTimer(app.host, WORKER_TIMER_ID, 50, None);
            drive(app);
            assert!(app.status.contains("最近决策：YES"), "{}", app.status);
            assert_eq!(app.text, "YES");
            assert!(app.visible);

            // Exercise the production capture/HDR check -> private pipe -> native HTTP route once.
            use std::{
                io::{Read, Write},
                net::TcpListener,
            };
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
            listener.set_nonblocking(true).unwrap();
            let server = std::thread::spawn(move || {
                let until = Instant::now() + Duration::from_secs(5);
                let mut stream = loop {
                    if let Ok((stream, _)) = listener.accept() {
                        break stream;
                    }
                    assert!(
                        Instant::now() < until,
                        "child never reached the loopback fixture"
                    );
                    std::thread::sleep(Duration::from_millis(10));
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut bytes = Vec::new();
                loop {
                    let mut part = [0u8; 4096];
                    let count = stream.read(&mut part).unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&part[..count]);
                    assert!(bytes.len() < 1_048_576);
                    if let Some(offset) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
                        let header = String::from_utf8_lossy(&bytes[..offset]).to_ascii_lowercase();
                        let length = header
                            .lines()
                            .find_map(|line| {
                                line.strip_prefix("content-length:")
                                    .map(|n| n.trim().parse::<usize>().unwrap())
                            })
                            .unwrap();
                        if bytes.len() < offset + 4 + length {
                            continue;
                        }
                        assert!(header.contains("authorization: bearer runtime-synthetic-key"));
                        assert!(header.starts_with("post /v1/chat/completions "));
                        let body: serde_json::Value =
                            serde_json::from_slice(&bytes[offset + 4..offset + 4 + length])
                                .unwrap();
                        assert_eq!(body["response_format"]["type"], "json_object");
                        assert!(body.to_string().contains("data:image/png;base64,"));
                        break;
                    }
                }
                let body = r#"{"choices":[{"message":{"content":"{\"choice\":\"option_2\"}"}}]}"#;
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            });
            app.hide();
            let executable = std::env::var_os("BLINK_VERDICT_TEST_EXE").unwrap();
            let mut command = Command::new(executable);
            command.arg("--decision-worker");
            let config = Config {
                provider: blink_verdict::config::Provider::LlamaCpp,
                endpoint,
                model: "blink-local".into(),
                vision: true,
                ..Config::default()
            };
            let request = Request {
                config,
                key: "runtime-synthetic-key".into(),
                preview: None,
                rect: [0, 0, 320, 200],
                hide: false,
                operation: 0,
            };
            app.worker =
                Some(Worker::spawn(command, Some(request), Duration::from_secs(5)).unwrap());
            app.busy = true;
            SetTimer(app.host, WORKER_TIMER_ID, 50, None);
            drive(app);
            server.join().unwrap();
            assert_eq!(app.text, "NO", "{}", app.status);
        }
    }
}
