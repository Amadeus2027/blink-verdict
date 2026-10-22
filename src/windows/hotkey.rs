use windows::Win32::UI::Input::KeyboardAndMouse::*;

pub fn parse(text: &str) -> Result<(HOT_KEY_MODIFIERS, u32), String> {
    let mut modifiers = MOD_NOREPEAT;
    let mut key = None;
    for token in text.split('+').map(|v| v.trim().to_ascii_uppercase()) {
        match token.as_str() {
            "CTRL" | "CONTROL" => modifiers |= MOD_CONTROL,
            "ALT" => modifiers |= MOD_ALT,
            "SHIFT" => modifiers |= MOD_SHIFT,
            "WIN" => modifiers |= MOD_WIN,
            other => {
                if key.is_some() {
                    return Err("快捷键只能包含一个普通按键".into());
                }
                key = Some(match other {
                    "SPACE" => VK_SPACE.0 as u32,
                    s if s.len() == 1 && s.as_bytes()[0].is_ascii_alphanumeric() => {
                        s.as_bytes()[0] as u32
                    }
                    s if s.starts_with('F') => {
                        let n: u32 = s[1..].parse().map_err(|_| "功能键格式错误".to_string())?;
                        if !(1..=24).contains(&n) {
                            return Err("功能键范围为 F1–F24".into());
                        }
                        VK_F1.0 as u32 + n - 1
                    }
                    _ => return Err("快捷键支持字母、数字、Space 或 F1–F24".into()),
                });
            }
        }
    }
    if modifiers == MOD_NOREPEAT {
        return Err("快捷键至少包含 Ctrl、Alt、Shift 或 Win 中的一项".into());
    }
    Ok((
        modifiers,
        key.ok_or_else(|| "快捷键缺少普通按键".to_string())?,
    ))
}
