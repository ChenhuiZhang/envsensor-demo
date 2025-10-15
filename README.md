# envsensor-demo

[![Build](https://github.com/ChenhuiZhang/envsensor-demo/actions/workflows/build.yml/badge.svg)](https://github.com/ChenhuiZhang/envsensor-demo/actions)
[![Latest Release](https://img.shields.io/github/v/release/ChenhuiZhang/envsensor-demo?color=blue)](https://github.com/ChenhuiZhang/envsensor-demo/releases)
[![License: MIT](https://img.shields.io/github/license/ChenhuiZhang/envsensor-demo)](LICENSE)
[![Last Commit](https://img.shields.io/github/last-commit/ChenhuiZhang/envsensor-demo)](https://github.com/ChenhuiZhang/envsensor-demo/commits)
[![Built with egui](https://img.shields.io/badge/UI-egui-blueviolet)](https://github.com/emilk/egui)
[![Rust Version](https://img.shields.io/badge/Rust-1.80+-orange)](https://www.rust-lang.org)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Windows-lightgrey)

---

A cross-platform **environment sensor demo** application built with [Rust](https://www.rust-lang.org) and [egui](https://github.com/emilk/egui).  
It demonstrates real-time serial communication, data visualization, and basic UI layout using the `egui` framework.

---

## ✨ Features

- 📡 Read sensor data from a serial port  
- 📊 Display live environmental metrics (CO, NO, etc.)
- 💾 Save the data in CSV file
- 🎨 Simple and responsive UI built with `egui`  
- ⚙️ Runs on both Linux and Windows  

---

## 🚀 Getting Started

### Prerequisites
- Rust **1.80+**
- A working serial device that outputs sensor data

### Build & Run
```bash
# Clone the repository
git clone https://github.com/ChenhuiZhang/envsensor-demo.git
cd envsensor-demo

# Build and run
cargo run --release --bin egui_demo

## 🧭 TODO
  
- [ ] Implement real-time chart updates      
- [ ] Package builds for Windows & Linux  
- [ ] Add unit tests for data parsing
- [ ] Implement slim UI  

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
