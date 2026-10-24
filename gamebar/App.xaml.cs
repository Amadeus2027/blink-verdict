using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Gaming.XboxGameBar;
using Windows.ApplicationModel;
using Windows.ApplicationModel.Activation;
using Windows.ApplicationModel.AppService;
using Windows.Foundation.Collections;
using Windows.Data.Json;
using Windows.UI.Core;
using Windows.UI.Xaml;
using Windows.UI.Xaml.Controls;

namespace BlinkVerdict.GameBar
{
    sealed partial class App : Application
    {
        private XboxGameBarWidget widget;
        private VerdictView view;
        private readonly List<AppServiceConnection> connections = new List<AppServiceConnection>();

        public App() { InitializeComponent(); }

        protected override void OnActivated(IActivatedEventArgs args)
        {
            // Only Game Bar's explicit user activation creates a window. IPC never activates one.
            var activation = args as XboxGameBarWidgetActivatedEventArgs;
            if (activation == null || !activation.IsLaunchActivation) return;
            var frame = new Frame();
            Window.Current.Content = frame;
            widget = new XboxGameBarWidget(activation, Window.Current.CoreWindow, frame);
            view = new VerdictView(widget);
            frame.Content = view;
            Window.Current.Closed += (s, e) => { view?.Hide(); view = null; widget = null; };
            Window.Current.Activate();
        }

        protected override void OnLaunched(LaunchActivatedEventArgs args)
        {
            // Normal Start-menu launch is an explicit setup action, not a decision trigger.
            Window.Current.Content = new TextBlock
            {
                Text = "BlinkVerdict Game Bar（实验性）\n\n按 Win+G，在组件菜单打开 BlinkVerdict，固定并开启点击穿透。\n\n将以下包家族名填写到主程序 → 设置 → … → 游戏中的显示：\n" + Package.Current.Id.FamilyName,
                Margin = new Thickness(28), TextWrapping = TextWrapping.Wrap, IsTextSelectionEnabled = true
            };
            Window.Current.Activate();
        }

        protected override void OnBackgroundActivated(BackgroundActivatedEventArgs args)
        {
            var details = args.TaskInstance.TriggerDetails as AppServiceTriggerDetails;
            if (details == null || details.Name != "BlinkVerdictBridge") return;
            var lifetime = args.TaskInstance.GetDeferral();
            var connection = details.AppServiceConnection;
            lock (connections) connections.Add(connection);
            long operation = 0;
            int closed = 0;
            connection.ServiceClosed += async (s, e) =>
            {
                Interlocked.Exchange(ref closed, 1);
                lock (connections) connections.Remove(connection);
                var current = view;
                if (current != null)
                {
                    try { await current.Dispatcher.RunAsync(CoreDispatcherPriority.Normal, () => current.Disconnect(Interlocked.Read(ref operation))); }
                    catch { }
                }
                lifetime.Complete();
            };
            connection.RequestReceived += async (s, e) =>
            {
                var request = e.GetDeferral();
                bool accepted = false;
                try
                {
                    var current = view;
                    if (current != null && e.Request.Message.TryGetValue("payload", out var payload) && payload is string json)
                    {
                        var message = JsonObject.Parse(json);
                        long requestOperation;
                        if (message.GetNamedNumber("protocol", 0) == 2 &&
                            long.TryParse(message.GetNamedString("operation", ""), out requestOperation) && requestOperation > 0)
                        {
                            Interlocked.Exchange(ref operation, requestOperation);
                            var completion = new TaskCompletionSource<bool>();
                            await current.Dispatcher.RunAsync(CoreDispatcherPriority.Normal, async () =>
                            {
                                try
                                {
                                    completion.TrySetResult(Volatile.Read(ref closed) == 0 &&
                                        await current.ApplyAsync(message, requestOperation));
                                }
                                catch { current.Fail(requestOperation); completion.TrySetResult(false); }
                            });
                            accepted = await completion.Task;
                        }
                    }
                    await e.Request.SendResponseAsync(new ValueSet { ["accepted"] = accepted, ["protocol"] = 2 });
                }
                catch { /* A closed client needs no UI or notification. */ }
                finally { request.Complete(); }
            };
        }
    }
}
