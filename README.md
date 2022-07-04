# NeteaseCloudMusicGtk4
> netease-cloud-music-gtk4 是基于 GTK4 + Libadwaita 构造的网易云音乐播放器，专为 Linux 系统打造，已在 openSUSE Tumbleweed + GNOME 环境下测试。

## 特点
- 稳定：专为 Linux 系统打造，相比官方版本拥有更好的兼容与稳定性。
- 极速：相比 Node/python 版，Rust + GTK 带给你如丝般的顺滑体验。
- 可靠：除了断网或网易 API 限制，不会出现运行时问题。
- 简洁：仿 GNOME Music 风格，GTK 原生界面，纯粹得令人发指。
- 轻量：安装文件不到 3 M，只需最简单的运行时依赖。

## 路线图
- [x] 发现页
- [x] 榜单页
- [x] 歌单详情页
- [x] 自适应皮肤
- [x] 网络代理
- [x] 扫码登陆
- [x] 验证码登陆
- [x] 播放栏
- [x] 多语言支持
- [x] 歌单页
- [x] 搜索页
- [x] 我的页
- [x] 首选项
- [x] Mpris2 绑定
- [x] 播放列表
- [x] 歌词
- [ ] 桌面歌词

## 运行依赖
> openssl, gstreamer, gstreamer-plugins-base, gstreamer-plugins-good, gstreamer-plugins-bad, gstreamer-plugins-ugly

## 安装
### openSUSE Tumbleweed
```bash
sudo zypper in netease-cloud-music-gtk
```
### openSUSE Leap
```bash
// 添加源
sudo zypper ar -f obs://multimedia:apps multimedia
// 安装
sudo zypper in netease-cloud-music-gtk
```

### Arch Linux
```bash
sudo pacman -Syu netease-cloud-music-gtk4
```

### flatpak
```
// 先下载 flatpak 安装包
sudo flatpak install com.gitee.gmg137.NeteaseCloudMusicGtk4-*.flatpak
```

### 从源码安装(不推荐)
> 编译依赖: opensssl、dbus、gtk4、gdk-pixbuf、libadwaita-1、gstreamer、gstreamer-base
```
// 下载源码
git clone https://github.com/gmg137/netease-cloud-music-gtk.git
cd netease-cloud-music-gtk

// 编译
meson _build
cd _build
ninja

// 安装
sudo ninja install
```

## FAQ
1. 为什么后台运行时没有托盘图标?
> 由于 GTK3 开始取消了托盘接口，所以目前不打算实现托盘功能。<br>
> **替代方案:**
> - Mpris 插件: GNOME 推荐 [Mpris Indicator Button](https://extensions.gnome.org/extension/1379/mpris-indicator-button/)，其它桌面可查找相应 Mpris 插件。
> - 直接点击启动图标，亦可唤醒程序。
2. 为什么点击歌曲后播放会有延迟?
> 对于未缓存歌曲会先缓存到本地后再进行播放，取决于音乐文件大小与网速，会有不同的播放延迟。
3. 音乐缓存目录在什么位置?
> 缓存位于用户主目录下 .cache/netease-cloud-music-gtk4 文件夹内。
4. 为什么每次启动时会变成静音?
> 参考了一些视频应用的做法，主要是为了防止突然播放时造成尴尬。

## 截图
![](./screenshots/discover.png)
![](./screenshots/discover-dark.png)
![](./screenshots/toplist.png)


## License
This project's source code and documentation is licensed under the  [GNU General Public License](COPYING) (GPL v3).

## 参考
- [Shortwave](https://gitlab.gnome.org/World/Shortwave)
- [gnome-music](https://gitlab.gnome.org/GNOME/gnome-music)
