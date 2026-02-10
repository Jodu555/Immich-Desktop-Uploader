# 📸 Immich Desktop Uploader

<p align="center">
  <img src="public/tauri.svg" width="60" alt="Tauri Logo" />
  <img src="src/assets/vue.svg" width="60" alt="Vue Logo" />
</p>

<p align="center">
  <strong>A cross-platform desktop application for automatic photo uploads to Immich</strong>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#installation">Installation</a> •
  <a href="#configuration">Configuration</a> •
  <a href="#building">Building</a>
</p>

---

<h2 id="features">✨ Features</h2>

- 🚀 **Automatic Uploads** - Schedule photo uploads with CRON expressions
- 📁 **Multi-Directory Support** - Monitor multiple folders simultaneously
- 🔄 **Duplicate Detection** - SHA1 checksum-based deduplication via Immich API
- ⏰ **Flexible Scheduling** - Set custom upload schedules (hourly, daily, weekly)
- 🖥️ **Cross-Platform** - Works on Windows, macOS, and Linux
- 🔔 **Real-time Logs** - Monitor upload status with live log feed
- 🎯 **Manual Trigger** - On-demand uploads when you need them
- 📂 **Recursive Scanning** - Include subdirectories in uploads
- 🎨 **Modern UI** - Clean, intuitive interface built with Vue 3

### Supported Image Formats

| Format | Extension       |
| ------ | --------------- |
| JPEG   | `.jpg`, `.jpeg` |
| PNG    | `.png`          |
| GIF    | `.gif`          |
| HEIC   | `.heic`         |
| WebP   | `.webp`         |
| TIFF   | `.tiff`         |

---

<h2 id="quick-start">🚀 Quick Start</h2>

### Prerequisites

Before you begin, ensure you have the following installed:

- [Node.js](https://nodejs.org/) (v18 or higher)
- [pnpm](https://pnpm.io/) (recommended) or npm
- [Rust](https://www.rust-lang.org/tools/install) (latest stable)

### 1. Clone the Repository

```bash
git clone https://github.com/Jodu555/Immich-Desktop-Uploader.git
cd immich-desktop-uploader
```

### 2. Install Dependencies

```bash
pnpm install
```

### 3. Run in Development Mode

```bash
pnpm tauri dev
```

This will start the development server and launch the desktop application.

---

<h2 id="installation">📦 Installation</h2>

### Build from Source

See the [Building](#building) section below.

---

<h2 id="configuration">⚙️ Configuration</h2>

### Setting Up Immich Connection

1. **Get your Immich API Key:**
   - Log in to your Immich web interface
   - Go to **Account Settings** → **API Keys**
   - Generate a new API key

2. **Configure the Application:**
   - Open Immich Desktop Uploader
   - Enter your Immich server URL (e.g., `https://immich.example.com`)
   - Paste your API key
   - Click **Test Connection** to verify

### Setting Up Upload Paths

1. Click **+ Add Path** to add a directory to monitor
2. Select a folder using the **Browse** button
3. Configure CRON expressions for automatic uploads (one per line):

| CRON Expression | Description       |
| --------------- | ----------------- |
| `10 * * * *`    | Every Hour at 10m |
| `30 * * * *`    | Every Hour at 30m |

4. Check **Include subdirectories** if you want to scan nested folders
5. Click **Save Configuration** to store your settings

### Starting the Scheduler

Once configured:

1. Click **Start Scheduler** to begin automatic uploads
2. The scheduler will check every minute for scheduled uploads
3. View real-time logs in the **Upload Logs** section

### Manual Upload

To upload immediately without waiting for the schedule:

1. Find the path you want to upload
2. Click the **Trigger** button
3. Watch the logs for upload progress

---

## 🛠️ Development

### Project Structure

```
immich-desktop-uploader/
├── src/                      # Vue.js frontend
├── src-tauri/               # Rust Tauri backend
```

### Available Scripts

| Command            | Description                   |
| ------------------ | ----------------------------- |
| `pnpm dev`         | Start development server      |
| `pnpm build`       | Build production frontend     |
| `pnpm preview`     | Preview production build      |
| `pnpm tauri dev`   | Run Tauri in development mode |
| `pnpm tauri build` | Build Tauri application       |

### Tech Stack

**Frontend:**

- [Vue 3](https://vuejs.org/) - Progressive JavaScript framework
- [TypeScript](https://www.typescriptlang.org/) - Type-safe JavaScript
- [Vite](https://vitejs.dev/) - Next generation frontend tooling

**Backend:**

- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Tauri](https://tauri.app/) - Framework for building desktop apps
- [Tokio](https://tokio.rs/) - Async runtime for Rust
- [Reqwest](https://docs.rs/reqwest/) - HTTP client for API calls

---

<h2 id="building">📱 Building</h2>

### Build for Current Platform

```bash
pnpm tauri build
```

This will create platform-specific binaries in `src-tauri/target/release/bundle/`.

### Build Outputs

| Platform | Output Location                             |
| -------- | ------------------------------------------- |
| Windows  | `src-tauri/target/release/bundle/msi/`      |
| macOS    | `src-tauri/target/release/bundle/dmg/`      |
| Linux    | `src-tauri/target/release/bundle/appimage/` |

### Cross-Platform Builds

Tauri supports cross-compilation. Refer to the [Tauri documentation](https://tauri.app/v1/guides/building/cross-platform/) for detailed instructions.

---

## 🔧 Troubleshooting

### Connection Issues

| Problem             | Solution                                                    |
| ------------------- | ----------------------------------------------------------- |
| "Failed to connect" | Verify your server URL includes `https://` or `http://`     |
| "Unauthorized"      | Check that your API key is valid and has proper permissions |
| "Server not found"  | Ensure your Immich server is accessible from your network   |

### Upload Failures

- Check the upload logs for specific error messages
- Ensure configured directories exist and are readable
- Verify you have sufficient disk space
- Check file permissions on the source directories

### Scheduler Issues

| Symptom               | Solution                                                                      |
| --------------------- | ----------------------------------------------------------------------------- |
| Scheduler won't start | Ensure configuration is saved first                                           |
| Uploads not happening | Verify CRON expressions are valid and scheduler is running                    |
| Duplicate uploads     | The app uses SHA1 checksums - this shouldn't happen unless files are modified |

---

## 🤝 Contributing

Contributions are welcome! Here's how you can help:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Follow the existing code style
- Write clear commit messages
- Test your changes thoroughly
- Update documentation as needed

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- [Immich](https://immich.app/) - The self-hosted photo and video backup solution
- [Tauri](https://tauri.app/) - Build smaller, faster, and more secure desktop applications
- [Vue.js](https://vuejs.org/) - The Progressive JavaScript Framework
- [Rust](https://www.rust-lang.org/) - A language empowering everyone to build reliable and efficient software

---

<p align="center">
  Made with ❤️ for the Immich community
</p>
