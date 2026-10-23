using System;
using System.Threading.Tasks;
using Microsoft.Gaming.XboxGameBar;
using Windows.Data.Json;
using Windows.Foundation;
using Windows.Graphics.Display;
using Windows.Storage.Streams;
using Windows.UI;
using Windows.UI.Xaml;
using Windows.UI.Xaml.Controls;
using Windows.UI.Xaml.Media;
using Windows.UI.Xaml.Media.Animation;
using Windows.UI.Xaml.Media.Imaging;

namespace BlinkVerdict.GameBar
{
    sealed class VerdictView : Page
    {
        private readonly XboxGameBarWidget widget;
        private readonly Grid panel = new Grid { Opacity = 0, IsHitTestVisible = false };
        private readonly Border pane = new Border { CornerRadius = new CornerRadius(24), IsHitTestVisible = false };
        private readonly Grid content = new Grid { HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center, RenderTransformOrigin = new Point(0.5, 0.5) };
        private readonly CompositeTransform transform = new CompositeTransform();
        private readonly TextBlock title = new TextBlock { Text = "B L I N K V E R D I C T", FontSize = 10, HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Top, Margin = new Thickness(12, 8, 12, 0) };
        private readonly TextBlock result = new TextBlock { TextAlignment = TextAlignment.Center };
        private readonly Image art = new Image { Stretch = Stretch.Fill, HorizontalAlignment = HorizontalAlignment.Left, VerticalAlignment = VerticalAlignment.Top };
        private readonly TextBlock footer = new TextBlock { FontSize = 10, HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Bottom, Margin = new Thickness(12, 0, 12, 8) };
        private readonly Viewbox fit;
        private readonly DispatcherTimer timer = new DispatcherTimer();
        private Storyboard animation;
        private bool fade;
        private bool fadingOut;
        private int generation;
        public bool IsProcessing { get; private set; }

        public VerdictView(XboxGameBarWidget widget)
        {
            this.widget = widget;
            IsHitTestVisible = false;
            content.RenderTransform = transform;
            content.Children.Add(result); content.Children.Add(art);
            fit = new Viewbox { Child = content, Margin = new Thickness(20, 12, 20, 28), Stretch = Stretch.Uniform, StretchDirection = StretchDirection.DownOnly };
            panel.Children.Add(pane); panel.Children.Add(fit); panel.Children.Add(footer);
            Content = panel;
            timer.Tick += (s, e) =>
            {
                timer.Stop();
                if (fade && !fadingOut)
                {
                    fadingOut = true;
                    double from = panel.Opacity;
                    animation?.Stop(); panel.Opacity = 0;
                    animation = new Storyboard();
                    Add(animation, panel, "Opacity", from, 0, 300);
                    animation.Begin();
                    timer.Interval = TimeSpan.FromMilliseconds(300); timer.Start();
                }
                else Hide();
            };
            widget.ClickThroughEnabledChanged += (s, e) => { if (!widget.ClickThroughEnabled) Hide(); };
            widget.PinnedChanged += (s, e) => { if (!widget.Pinned) Hide(); };
        }

        public void Hide()
        {
            generation++; IsProcessing = false;
            timer.Stop(); animation?.Stop(); animation = null;
            panel.Opacity = 0; art.Source = null;
            transform.ScaleX = transform.ScaleY = 1; transform.TranslateY = 0;
        }

        public async Task<bool> ApplyAsync(string json)
        {
            var message = JsonObject.Parse(json);
            string action = message.GetNamedString("action");
            Hide();
            if (!widget.Pinned || !widget.ClickThroughEnabled || !widget.Visible || widget.GameBarDisplayMode != XboxGameBarDisplayMode.PinnedOnly) return false;
            if (action == "hide") { await Task.Delay(40); return true; }
            if (action != "show" && action != "processing") return false;
            IsProcessing = action == "processing";
            int requestGeneration = generation;
            bool fullscreen = message.GetNamedBoolean("fullscreen");
            double scale = DisplayInformation.GetForCurrentView().RawPixelsPerViewPixel;
            var size = new Size(message.GetNamedNumber("width") / scale, message.GetNamedNumber("height") / scale);
            widget.MaxWindowSize = new Size(Math.Max(1600, size.Width), Math.Max(1000, size.Height));
            if (!await widget.TryResizeWindowAsync(size)) return false;
            if (fullscreen) await widget.CenterWindowAsync();
            if (generation != requestGeneration) return false;
            // IPC never activates a window, launches Game Bar, or simulates input.
            // The fullscreen surface stays transparent, including while processing.
            var background = ColorFromHex(message.GetNamedString("backgroundColor"));
            background.A = (byte)message.GetNamedNumber("widgetBackgroundOpacity", 180);
            pane.Background = fullscreen ? null : new SolidColorBrush(background);
            result.Text = message.GetNamedString("text");
            result.FontFamily = new FontFamily(message.GetNamedString("font"));
            result.FontSize = message.GetNamedNumber("fontSize") / scale;
            result.Foreground = new SolidColorBrush(ColorFromHex(message.GetNamedString("textColor")));
            title.Foreground = footer.Foreground = new SolidColorBrush(Color.FromArgb(255, 166, 168, 178));
            title.FontFamily = footer.FontFamily = result.FontFamily;
            title.Visibility = footer.Visibility = fullscreen ? Visibility.Collapsed : Visibility.Visible;
            fit.Margin = fullscreen ? new Thickness(24) : new Thickness(20, 12, 20, 28);
            footer.Text = message.GetNamedString("hotkey");
            await SetArtAsync(message, action == "show");
            if (generation != requestGeneration) return false;
            double opacity = message.GetNamedNumber("opacity") / 255.0;
            string effect = action == "show" ? message.GetNamedString("animation") : "None";
            fade = effect != "None"; fadingOut = false; panel.Opacity = opacity;
            animation = new Storyboard();
            if (effect == "Fade" || effect == "Pop" || effect == "Slide") Add(animation, panel, "Opacity", 0, opacity, 220);
            if (effect == "Pop") { Add(animation, transform, "ScaleX", 0.78, 1, 220); Add(animation, transform, "ScaleY", 0.78, 1, 220); }
            if (effect == "Slide") Add(animation, transform, "TranslateY", 28, 0, 220);
            if (effect == "Flash")
            {
                var pulse = new DoubleAnimationUsingKeyFrames();
                pulse.KeyFrames.Add(new DiscreteDoubleKeyFrame { KeyTime = KeyTime.FromTimeSpan(TimeSpan.Zero), Value = opacity });
                pulse.KeyFrames.Add(new DiscreteDoubleKeyFrame { KeyTime = KeyTime.FromTimeSpan(TimeSpan.FromMilliseconds(80)), Value = opacity * 0.65 });
                pulse.KeyFrames.Add(new DiscreteDoubleKeyFrame { KeyTime = KeyTime.FromTimeSpan(TimeSpan.FromMilliseconds(160)), Value = opacity });
                Storyboard.SetTarget(pulse, panel); Storyboard.SetTargetProperty(pulse, "Opacity"); animation.Children.Add(pulse);
            }
            if (animation.Children.Count > 0) animation.Begin();
            double duration = action == "processing" ? 140000 : message.GetNamedNumber("durationMs");
            if (duration > 0)
            {
                timer.Interval = TimeSpan.FromMilliseconds(Math.Max(1, duration - (fade ? 300 : 0))); timer.Start();
            }
            return true;
        }

        private async Task SetArtAsync(JsonObject message, bool show)
        {
            art.Visibility = Visibility.Collapsed; result.Visibility = Visibility.Visible;
            content.Width = content.Height = double.NaN; content.Clip = null;
            fit.MaxHeight = double.PositiveInfinity;
            if (!show) return;
            string custom = message.GetNamedString("customArt");
            if (custom.Length > 0)
            {
                var bitmap = new BitmapImage();
                using (var stream = new InMemoryRandomAccessStream())
                {
                    using (var writer = new DataWriter(stream))
                    {
                        writer.WriteBytes(Convert.FromBase64String(custom)); await writer.StoreAsync();
                        stream.Seek(0); await bitmap.SetSourceAsync(stream);
                    }
                }
                if (bitmap.PixelWidth > 4096 || bitmap.PixelHeight > 4096) return;
                art.Source = bitmap; art.RenderTransform = null;
                content.Width = art.Width = bitmap.PixelWidth; content.Height = art.Height = bitmap.PixelHeight;
            }
            else
            {
                string style = message.GetNamedString("artStyle");
                string word = result.Text.ToUpperInvariant();
                if (style == "Plain" || (word != "YES" && word != "NO")) return;
                int column = style == "Ink" ? 0 : style == "Cyberpunk" ? 1 : 2;
                int row = word == "YES" ? 0 : 1;
                content.Width = content.Height = 512;
                content.Clip = new RectangleGeometry { Rect = new Rect(0, 0, 512, 512) };
                art.Width = 1536; art.Height = 1024;
                art.RenderTransform = new TranslateTransform { X = -column * 512, Y = -row * 512 };
                art.Source = new BitmapImage(new Uri("ms-appx:///Assets/wordart.png"));
            }
            fit.MaxHeight = result.FontSize * 3;
            result.Visibility = Visibility.Collapsed; art.Visibility = Visibility.Visible;
        }

        private static void Add(Storyboard board, DependencyObject target, string property, double from, double to, int milliseconds)
        {
            var transition = new DoubleAnimation { From = from, To = to, Duration = new Duration(TimeSpan.FromMilliseconds(milliseconds)), EasingFunction = new CubicEase { EasingMode = EasingMode.EaseOut } };
            Storyboard.SetTarget(transition, target); Storyboard.SetTargetProperty(transition, property); board.Children.Add(transition);
        }

        private static Color ColorFromHex(string text)
        {
            uint n = Convert.ToUInt32(text.TrimStart('#'), 16);
            return Color.FromArgb(255, (byte)(n >> 16), (byte)(n >> 8), (byte)n);
        }
    }
}
