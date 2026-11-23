<div align="center">
  <img src="obscat.png" alt="Obscat Logo" width="200">
  
  <h1>Obscat</h1>
  <p><strong>Fast, lightweight, zero-build container engine manager built with Rust.</strong></p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/License-GNU_General_Public_License_v3.0-5A0E3B" alt="License" />
</p>
</div>

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
* we supported linux only with Architecture for now
* aarch64-gnu, x86_64-gnu
* installing youki in the github youki-dev/youki, or manual in instalation section below.

### Installation

You can install `Obscat container engine` instantly using our automated setup script. Run the following command in your terminal:

```bash
curl -fsSL https://raw.githubusercontent.com/xlients112/obscat/main/install.sh | sh
```
---
***BINARY***
* obscat
* obscatd
* youki

## Usage Quickstart

Here are a few basic commands to get you up and running:

### Starting daemon engine
```bash
obscat daemon-start
```
check the daemon
```bash
obscat daemon-check
```

### Verify Installation
```bash
obscat version
```

### Pull an Image
for now we have only docker-hub image repository to download/pull the images you want, usage:
```bash
obscat image pull docker-hub node:24
```

### Create your Container
Before creating container, create manifest ``obscat.toml`` inside your folder project

example
```toml
[package]
name = "your-package-name" # your container name
version = "0.1.0"

[container]
image = "busybox:latest" #your image
source = "." #your source application
destination = "/app" #destination
command = ["/bin/sh", "-c", "httpd -f -p 8080 -h /app"] #some command
workdir = "/app"
ports = "8080:80" #setting your container port
network_mode = "bridge" #netowrk mode, delete this if you want using default
ip_address = "10.88.0.15" #container IP (opsional)

[container.env] #container ENV
APP_ENV = "production"
MY_API_KEY = "API_key_xxxxx"
PORT = "8080"
```

or using templates command with:
```bash
obscat init-manifest
```

### Create your container
run this command inside your folder project with obscat.toml
```bash
cd my-project
```
```bash
obscat create .
```

Or, in the outside to point your folder in the command
```bash
obscat create my-project
```

### Check your Container list
```bash
obscat list
```

### Check your container state.
```bash
obscat state container_id
```

### Deleting your container
```bash
obscat delete my-project
```
---

## Deployment
A deployment functions as a deployment space folder on the host, it creates and runs all container instances listed in ``obscat.toml`` including multiple instances of a container—along with your application source code.

### How It Works
***Create your deployment folder:***
use this command, to automatic create deployment folder and ``obscat.toml`` example template.
```bash
obscat init-deployment your-deployment
```
note: name of the folder `my-deployment` is deployment ID for register point to manage your deployment space.

---
***add your source application:***
copy your folder source application inside service folder `my-deployment/service`

***Configurate your obscat.toml***
example:
```toml
[package]
name = "my-deployment" # ensure this package name same with your deployment folder
version = "0.1.0"

[service.yourapps]
image = "busybox:latest" # Register the image you have installed.
destination = "/www" # destination path
workdir = "/www"  # work directory
ports = "8080:80" # custom port
command = ["/bin/httpd", "-f", "-p", "80", "-h", "/www"] # your application command

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
to deploy your service application inside deployment folder, use command in outside deployment folder:
```bash
obscat deploy your-deployment-folder
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
Undeploy is automatic deleted your container service list instanly use by reading the lockfile manifest ``obscat.lock``, cleaning the mounting overlayFS, deleting the bundles folder and your source at inside your container, and clean the networking settings.

> ### [CAUNTION]
> **VERY IMPORTANT, obscat daemon undeployed by reading the obscat.lock, ensure you are not DELETING obscat.lock before undeployed!.**
> The daemon will become confused if `obscat.lock` is missing; however, if you successfully perform an undeploy, `obscat.lock` will be automatically deleted.

***usage***
inside deployment folder:
```bash
obscat undeploy .
```
in the outside:
```bash
obscat undeploy your-deployment-folder
```

---
### Benchmark
#### Deployment Performance

| Metric | Result |
|---|---:|
| Containers Deployed | 11 Containers |
| Deployment Time (Wall Clock) | **1.101 seconds** |
| CLI Peak RAM Usage | 6.00 MB |
| Command Execution Status | Success |

#### Daemon Resource Consumption

| Condition | CPU Usage | RAM Usage |
|---|---:|---:|
| Before Deployment (Idle) | 0.0% | 5.68 MB |
| During Deployment (Peak) | 0.00% | 5.79 MB |
| After Deployment (Running) | 0.0% | 5.79 MB |

#### Memory Overhead

| Metric | Result |
|---|---:|
| Idle RAM | 5.68 MB |
| Running RAM | 5.79 MB |
| RAM Increase | **+0.11 MB** |

**See full benchmark:** [Benchmark](https://github.com/xlients112/obscat/tree/main/benchmark)

---

## Contributing

Contributions are welcome! Whether it is reporting a bug, proposing new features, or submitting a pull request, your help makes `obscat` better.
---

## 📄 License

Distributed under the **GNU General-Public-License V3.0 LICENSE**. See `LICENSE` for more information.
