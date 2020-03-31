# netease-cloud-music-gtk
[![Actions Status](https://github.com/gmg137/netease-cloud-music-gtk/workflows/CI/badge.svg)](https://github.com/gmg137/netease-cloud-music-gtk/actions)
> netease-cloud-music-gtk 是基于 Rust + GTK 开发的网易云音乐播放器，专为 Linux 系统打造，已在 openSUSE Tumbleweed + GNOME 环境下测试。

## 特点
- 稳定：专为 Linux 系统打造，相比官方版本拥有更好的兼容与稳定性。
- 极速：相比 Node/python 版，Rust + GTK 带给你如丝般的顺滑体验。
- 可靠：除了断网或网易 API 限制，不会出现运行时问题。
- 简洁：仿 GNOME Music 风格，GTK 原生界面，纯粹得令人发指。
- 轻量：安装文件不到 2 M，只需最简单的运行时依赖。

## 功能
- 网易邮箱/手机账号登录
- 私人歌单管理
- 个性推荐
- 私人 FM
- 音乐云盘
- 热门排行榜
- 歌曲搜索
- 简易歌词
- 桌面歌词(依赖于 [OSDLyrics](https://github.com/osdlyrics/osdlyrics))
- 热门歌单
- 新碟上架

## 运行依赖
> openssl, curl, gstreamer, gstreamer-plugins-base, gstreamer-plugins-good, gstreamer-plugins-bad, gstreamer-plugins-ugly

## 安装
- 直接到 [Release](https://github.com/gmg137/netease-cloud-music-gtk/releases) 页面下载 RPM/DEB 包安装。

## 从源码编译/打包
```
// openSUSE 安装依赖
sudo zypper in git gcc curl dbus-1-devel gtk3-devel libopenssl-1_1-devel gstreamer-devel \
      gstreamer-plugins-bad gstreamer-plugins-bad-devel \
      gstreamer-plugins-base gstreamer-plugins-base-devel \
      gstreamer-plugins-good gstreamer-plugins-ugly cairo-devel
```
```
// ubuntu 安装依赖
sudo apt install git gcc libdbus-1-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
      gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
      gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly \
      gstreamer1.0-libav libgtk-3-dev libatk1.0-dev libcogl-pango-dev \
      libssl-dev libcairo2-dev libgstreamer-plugins-bad1.0-dev
```
```
git clone https://github.com/gmg137/netease-cloud-music-gtk.git
cd netease-cloud-music-gtk

// 编译
cargo build --release
// 编译指定 gtk 版本(Leap 15.1 / Ubuntu 18.04)
cargo build --release --no-default-features --features gtk_3_18

// 打包 rpm
cargo install cargo-rpm
cargo rpm build

// 打包 deb
cargo install cargo-deb
cargo deb
```

## 截图
![home](https://user-images.githubusercontent.com/6460323/74423902-fa996900-4e8b-11ea-915f-a4ec40bd2982.jpg)
![found](https://user-images.githubusercontent.com/6460323/74421939-c8d2d300-4e88-11ea-9b93-962ae80f5a11.png)
![mine](https://user-images.githubusercontent.com/6460323/74424004-29afda80-4e8c-11ea-9c16-af3f25525c9c.jpeg)

## 参考
- [podcasts](https://gitlab.gnome.org/World/podcasts)
- [gnome-music](https://gitlab.gnome.org/GNOME/gnome-music)
- [musicbox](https://github.com/darknessomi/musicbox)
- [NeteaseCloudMusicRustApi](https://github.com/Itanq/NeteaseCloudMusicRustApi)
