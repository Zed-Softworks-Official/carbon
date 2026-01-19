# Carbon - Video Downloader for DaVinci Resolve

A terminal-based video downloader and converter built with Rust and Ratatui. Download videos from YouTube and Twitch, then automatically convert them to a format compatible with DaVinci Resolve on Linux.

## Features

- 🎥 Download videos from YouTube and Twitch using yt-dlp
- 🔄 Automatic conversion to DaVinci Resolve compatible format (MP4 with PCM audio)
- ⚡ Multiple concurrent downloads (configurable, default: 3)
- 📊 Real-time progress tracking for downloads and conversions
- 🎨 Beautiful terminal UI with Ratatui
- ⚙️ Configurable output directory and quality settings
- 🚀 Async processing with Tokio for optimal performance

## Prerequisites

The following tools must be installed on your system:

- **yt-dlp** - For downloading videos
- **ffmpeg** - For video conversion
- **ffprobe** - For video analysis (usually comes with ffmpeg)

### Installing Prerequisites

**Arch Linux:**
```bash
sudo pacman -S yt-dlp ffmpeg
```

**Ubuntu/Debian:**
```bash
sudo apt install yt-dlp ffmpeg
```

**Fedora:**
```bash
sudo dnf install yt-dlp ffmpeg
```

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/carbon`.

You can optionally install it to your system:

```bash
cargo install --path .
```

## Usage

Run the application:

```bash
cargo run --release
# or if installed:
carbon
```

### Keyboard Controls

**Main View:**
- `a` - Add a new URL to the download queue
- `↑/↓` - Navigate through the job list
- `d` - Delete selected job (only non-active jobs)
- `c` - Clear completed jobs
- `q` - Quit application

**Input Mode (when adding URL):**
- `Ctrl+V` - Paste URL from clipboard
- `Enter` - Submit URL and start download
- `Esc` - Cancel input and return to main view
- Type normally to enter URL manually

### How It Works

1. Press `a` to open the URL input
2. Enter a YouTube or Twitch URL (or press `Ctrl+V` to paste from clipboard)
3. Press `Enter` to add to queue
4. The application will:
   - Download the video using yt-dlp
   - Automatically convert it to DaVinci Resolve compatible format
   - Save it to your configured output directory

The converted videos will have PCM audio (16-bit, 48kHz) which is compatible with DaVinci Resolve on Linux, where AAC audio codec support is limited.

## Configuration

Configuration is stored at `~/.config/carbon/config.toml`.

Default configuration:
```toml
output_directory = "~/Videos/DaVinci"
max_concurrent_downloads = 3
default_quality = "best"
auto_convert = true
```

### Configuration Options

- `output_directory` - Where converted videos are saved
- `max_concurrent_downloads` - Number of simultaneous downloads (1-10)
- `default_quality` - Video quality: "best", "1080p", "720p", or "480p"
- `auto_convert` - Automatically convert videos after download (true/false)

## Technical Details

### Video Conversion

The application converts videos using FFmpeg with the following parameters:

```bash
ffmpeg -i input.mp4 \
       -c:v copy \
       -c:a pcm_s16le \
       -ar 48000 \
       output.mp4
```

This:
- Copies the video stream without re-encoding (fast)
- Converts audio to PCM 16-bit little-endian
- Sets sample rate to 48kHz (standard for video)

### Why PCM Audio?

DaVinci Resolve on Linux has limited AAC audio codec support. Converting to PCM ensures maximum compatibility while preserving video quality by copying the video stream without re-encoding.

## Project Structure

```
src/
├── main.rs         # Entry point
├── app.rs          # Application state and event handling
├── ui.rs           # Terminal UI rendering
├── downloader.rs   # yt-dlp wrapper
├── converter.rs    # FFmpeg wrapper
├── queue.rs        # Job queue with concurrency control
├── config.rs       # Configuration management
└── models.rs       # Data structures
```

## Troubleshooting

### "yt-dlp: command not found"

Make sure yt-dlp is installed and in your PATH. See Prerequisites section.

### "ffmpeg: command not found"

Make sure ffmpeg is installed and in your PATH. See Prerequisites section.

### Videos won't play in DaVinci Resolve

Ensure the conversion completed successfully. Check the job list for any errors. The converted files should be in your configured output directory with `_davinci.mp4` suffix.

### Downloads are slow

- Check your internet connection
- Try reducing `max_concurrent_downloads` in the config
- Some video hosts rate-limit downloads

## Development

Build in debug mode:
```bash
cargo build
```

Run tests:
```bash
cargo test
```

## License

See LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues.

## Acknowledgments

- Built with [Ratatui](https://github.com/ratatui/ratatui) - Terminal UI framework
- Uses [yt-dlp](https://github.com/yt-dlp/yt-dlp) - Video downloader
- Uses [FFmpeg](https://ffmpeg.org/) - Video converter
