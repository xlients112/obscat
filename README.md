# Obscat, a new line container engine
<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/License-GNU_General_Public_License_v3.0-5A0E3B" alt="License" />
</p>

## Description

**Obscat container engine manager** fast, lightweight, zero build, and high-performance to create container and deployment can make 10+ container in one command on your system.

Built with rust and integrated with the low-level OCI 'youki' backend, Deployment mode with zero-build with the OverlayFS system. 

---

## Key Features

* **OCI Standard Compliant** – Full compatibility with Open Container Initiative (OCI) images and runtimes.
* **Image manager** – Manage your own base images for use in multiple containers with docker-hub repository.
* **Resource Efficient** – Minimal CPU and memory footprint compared to traditional container daemons.
* **Zero build with deployment mode** –  Register your deployment folder to make more than 1 instance container in one command without build anything.
---

## Getting Started

### Prerequisites
* we supported linux only with Architecture for now:
* aarch64-gnu, x86_64-gnu

### Installation

You can install `Obscat container manager` instantly using our automated setup script. Run the following command in your terminal:

```bash
curl -fsSL https://raw.githubusercontent.com/xlients112/obscat/main/install.sh | sh
```
---

## Usage Quickstart

Here are a few basic commands to get you up and running:

### Starting daemon engine
```bash
obscat daemon-start
```
or manualy (auto connect)
```bash
obscatd &
```

### Verify Installation
```bash
obscat --version
```

### Pull an Image
```bash
obscat pull alpine:latest
```

### Run a Container
```bash
obscat create --id container_id
```
### Check your container state.
```bash
obscat state container_id
```
---

## Deployment
A deployment functions as a deployment space folder on the host, it creates and runs all container instances listed in `manifest.toml` including multiple instances of a container—along with your application source code.

### How It Works
***Create your deployment folder:***
use this command, to automatic create deployment folder and `manifest.toml` example template.
```bash
obscat init my-deployment
```
note: name of the folder `my-deployment` is deployment ID for register point to manage your deployment space.

---
***add your source application:***
copy your folder source application inside service folder `my-deployment/service`

***Configurate your manifest.toml***
example:
```toml
[package]
name = "my-deployment" //ensure this package name same with your deployment folder
version = "0.1.0"

[service.yourapps]
image = "busybox:latest" //Register the image you have installed.
destination = "/www" //destination path
workdir = "/www"  // work directory
ports = "8080:80" //custom port
command = ["/bin/httpd", "-f", "-p", "80", "-h", "/www"] // your application command

[service.yourapps.env]
APP_ENV = "production"
```

### Example deployment Layout
```text
my-project/
├── manifest.toml.   # your container config maniifest
└── service/
    ├── api/main.rs...            # Your apps in container_1
    ├── worker/main.go...         # Your apps in container_2
    └── web/index.html...         # Your apps in container_3
```

### Deploying your service
to deploy your service application inside deployment folder, use command:
```bash
obscat deploy ./your-deployment-folder
```
or inside:
```bash
cd your-deployment-folder
```
```bash
obscat deploy .
```
and using spesific folder:
```bash
obscat deploy /home/youruser/your-deployment-folder
```

### Undeploy
Undeploy is automatic deleted your container service list instanly (this feature is experimental)
however, because there is a risk that some processes or mount points might still be dependent on it.

***usage:***
```bash
obscat undeploy your-deployment-id
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

Distributed under the **GNU General-Public-License V3.0 LICENSE**. See `LICENSE` for more information.
