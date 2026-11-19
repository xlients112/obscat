# Obscat, a new line container engine
<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/License-GNU_General_Public_License_v3.0-5A0E3B" alt="License" />
</p>

## Description

**Obscat container engine manager** fast, lightweight, zero build, and high-performance to create container and deployment can make 10+ container in one command on your system.

Built with rust and integrated with the low-level OCI 'youki' backend, Deployment mode with zero-build with the OverlayFS system. 

---

## ✨ Key Features

* **OCI Standard Compliant** – Full compatibility with Open Container Initiative (OCI) images and runtimes.
* **Image manager** – Manage your own base images for use in multiple containers.
* **Resource Efficient** – Minimal CPU and memory footprint compared to traditional container daemons.
* **Zero build with deployment mode** –  Register your deployment folder to make more than 1 instance container in one command without build anything.
---

## Getting Started

### Prerequisites
* we supported linux only with Architecture:
* aarch64-gnu, x86_64-gnu

### Installation

You can install `Obscat container manager` instantly using our automated setup script. Run the following command in your terminal:

```bash
curl
```

---

## Usage Quickstart

Here are a few basic commands to get you up and running:

### Verify Installation
```bash
xserve --version
```

### Pull an Image
```bash
xserve pull alpine:latest
```

### Run a Container
```bash
xserve create --id container_id
```

### List Running Containers
```bash
xserve ps
```
```bash
xserve state container_id
```

## Deployment
Deployment is useful for registering your source application into the deployment mode folder, this is useful so that the engine can read the manifest and register your application source in more than 1 container.

### How It Works
1. **Drop-in Service Directory:** Organize your application source codes into a project directory alongside a single `manifest.toml` file—anywhere on your filesystem.
2. **Manage your manifest.toml**
manifest.toml config works to register your application, select the image, mount apps and destination folder in the container.
4. **Isolated Multi-Service Execution:** Each service defined in the manifest is spawned as an independent, isolated container instance powered by `youki`, sharing a single base image layer with near-zero disk overhead and millisecond cold-start times.

### Example deployment Layout
```text
my-project/
├── manifest.toml.   # your container config
└── service/
    ├── api/          # Your apps in container_1
    ├── worker/       # Your apps in container_2
    └── web/          # Your apps in container_3
```


---

## ⚙️ Configuration

`xserve` works out of the box with zero configuration. However, you can easily customize its behavior by modifying the global configuration file:

```bash
nano ~/.config/xserve/config.toml
```

---

## 🤝 Contributing

Contributions are welcome! Whether it is reporting a bug, proposing new features, or submitting a pull request, your help makes `xserve` better.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

---

## 📄 License

Distributed under the **MIT License**. See `LICENSE` for more information.
