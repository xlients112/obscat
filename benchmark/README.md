# Obscat Container Engine — Benchmark

This page contains benchmark results for Obscat Container Engine, focusing on daemon resource consumption and container deployment performance.

The primary goal is to measure the overhead introduced by `obscatd`, independently from the workloads running inside containers.

## Benchmark Results

### v0.1.3

#### Test Environment

| Property | Value |
|---|---|
| Test Date | November 22, 2026 |
| Environment | Cloud VPS |
| Operating System | Linux |
| Kernel | 6.8.0-147-generic |
| CPU | Intel Xeon E312xx (Sandy Bridge) |
| CPU Frequency | 2.0 GHz |
| Total Host RAM | 1.96 GiB |
| BIOS | RHEL 9.8.9 PC (Q35 + ICH9) |
| Target Directory | `/root/test` |
| Obscat Daemon PID | 3340 |
| Exit Status | Success (0) |

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

> Daemon resource measurements were collected at 100 ms sampling intervals.

-- full source at file: Benchmark-tes-v0.1.3.txt

## Benchmark Methodology

The benchmark focuses on the resource overhead of the Obscat daemon rather than the resource consumption of individual container workloads.

The following metrics are monitored:

- **Daemon CPU Usage:** CPU utilization during idle, deployment, and running phases.
- **Daemon RAM Usage:** Memory consumption before, during, and after deployment.
- **CLI Memory Usage:** Peak memory consumption of the Obscat CLI process.
- **Deployment Time:** Total wall-clock time required to complete the deployment command.
- **Container Count:** Number of successfully detected containers after deployment.

## Benchmark tools
- **benchmark.sh** Using a linux tools to read and monitoring benchmark for deployment command ```obscat deploy deploy-file```,
- **stres.sh & stres-net.sh** Using ``apache2-utils`` and ``wrx`` to send over 2k request and 10+ concurent to testing container nettwork stress, but network test is curently not testing now.
- **benchmark-self in yout host** if you interesting to compare a benchmark, you can download, edit, or whatever you do to a looking obscat benchmark.

## Reproducibility

Benchmark source code and test procedures should be publicly available so that anyone can inspect the measurement methodology and reproduce the results.

For meaningful comparisons, use the same host environment, kernel, measurement intervals, and deployment conditions.

## Releases

| Version | Containers | Deployment Time | Daemon RAM (Peak) |
|---|---:|---:|---:|
| v0.1.3 | 11 | 1.101 s | 6 MB |

---

**Obscat Container Engine**

*Lightweight container orchestration with minimal daemon overhead.*