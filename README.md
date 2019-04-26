## netease-cloud-music-gtk
> netease-cloud-music-gtk 是基于 Rust + GTK 开发的网易云音乐播放器，计划只支持 Linux 系统，已在 openSUSE Tumbleweed + GNOME 环境下测试。

### 特点
- 极速：相比 Node/python 版，Rust 速度可谓一骑绝尘
- 稳定：除了网速或网易 API 限制，基本不会出现运行问题
- 简洁：仿 GNOME Music 风格，GTK 原生界面，纯粹得令人发指
- 简单：极小的编译与运行依赖

### 功能
- 网易邮箱/手机账号登陆
- 个人歌单
- 私人 FM
- 排行榜
- 歌曲搜索
- 歌词(依赖于 OSDLyrics)
- 热门歌单(8个)
- 推荐歌单(4个)

### 依赖
> openssl, curl, gstreamer, gstreamer-plugins-base, gstreamer-plugins-good, gstreamer-plugins-bad, gstreamer-plugins-ugly

### 安装
- 直接下载 RPM 包安装或解压 tar.xz 包手动复制到相应目录

### 从源码编译/打包
```
git clone https://github.com/gmg137/netease-cloud-music-gtk.git
cd netease-cloud-music-gtk

// 编译
cargo build --release

// 打包 rpm
cargo install cargo-rpm
cargo rpm build

// 打包 deb
cargo install cargo-deb
cargo deb
```

### 截图
![2019-04-11 17-18-01 的屏幕截图](https://user-images.githubusercontent.com/6460323/55945759-01f55200-5c7e-11e9-9a91-606a4656555e.png)
![2019-04-11 17-18-22 的屏幕截图](https://user-images.githubusercontent.com/6460323/55945765-04f04280-5c7e-11e9-9f38-242524aedd66.png)
![2019-04-11 17-18-44 的屏幕截图](https://user-images.githubusercontent.com/6460323/55945774-07529c80-5c7e-11e9-9dbd-eefa9e387096.png)


### 参考
- [podcasts](https://gitlab.gnome.org/World/podcasts)
- [gnome-music](https://gitlab.gnome.org/GNOME/gnome-music)
- [musicbox](https://github.com/darknessomi/musicbox)
