//
// web_login.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under the terms of the GPL-3.0-or-later license.
//

//! 网易云官方网页登录 / cookie 导入。
//!
//! 短信验证码登录会被网易风控拦截（-462 要求人机验证），扫码登录也可能返回
//! -462 / 8821。这里提供兜底：直接打开官方登录页，登录成功后从页面的
//! cookie 里取回 MUSIC_U，交给主流程校验并保存。
//!
//! 编译时带 `webview` feature 时内嵌 WebKitGTK 页面并自动取 cookie；
//! 否则退化为「在浏览器里登录 + 粘贴 cookie」。

use async_channel::Sender;
use gettextrs::gettext;
use gtk::{gio, prelude::*};
use log::warn;

use crate::{application::Action, model::UserMenuChild, ncmapi::NcmClient};

#[cfg(feature = "webview")]
use gtk::glib;
#[cfg(feature = "webview")]
use log::debug;

/// 官方网页登录页（与网页版客户端登录弹窗同一份页面）
pub const LOGIN_URL: &str = "https://music.163.com/login?targetUrl=%2F";

/// 需要从页面 cookie 中取回的关键项
#[cfg(feature = "webview")]
const WANTED_COOKIES: [&str; 8] = [
    "MUSIC_U",
    "MUSIC_A",
    "__csrf",
    "NMTID",
    "_ntes_nnid",
    "_ntes_nuid",
    "os",
    "appver",
];

/// 打开登录窗口：能内嵌网页就内嵌，否则退化为粘贴 cookie
pub fn show_login_window(parent: &gtk::Window, sender: Sender<Action>) {
    #[cfg(feature = "webview")]
    {
        show_webview_window(parent, sender);
    }
    #[cfg(not(feature = "webview"))]
    {
        show_cookie_dialog(parent, sender);
    }
}

/// 用 MUSIC_U / cookie 串直接登录（粘贴方式与网页取回共用）
pub fn login_with_cookies(sender: &Sender<Action>, text: &str) -> bool {
    let cookie_jar = NcmClient::parse_cookie_string(text);
    let has_music_u = match "https://music.163.com".parse() {
        Ok(url) => cookie_jar
            .get_by_name(&url, "MUSIC_U")
            .map(|c| !c.value().is_empty())
            .unwrap_or(false),
        Err(_) => false,
    };
    if !has_music_u {
        return false;
    }
    let _ = sender.send_blocking(Action::CheckLogin(UserMenuChild::Qr, cookie_jar));
    true
}

/// 在系统浏览器中打开官方登录页
pub fn open_in_browser() {
    if let Err(err) =
        gio::AppInfo::launch_default_for_uri(LOGIN_URL, None::<&gio::AppLaunchContext>)
    {
        warn!("打开浏览器失败: {:?}", err);
    }
}

/// 无 webview 时的兜底：浏览器登录 + 粘贴 cookie
fn show_cookie_dialog(parent: &gtk::Window, sender: Sender<Action>) {
    let entry = gtk::Entry::builder()
        .placeholder_text(gettext("Paste MUSIC_U or the whole cookie string"))
        .hexpand(true)
        .build();

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    content.append(
        &gtk::Label::builder()
            .label(gettext(
                "Sign in on the page opened in your browser, then paste the cookie here \
                 (DevTools -> Application -> Cookies -> music.163.com -> MUSIC_U).",
            ))
            .wrap(true)
            .xalign(0.0)
            .build(),
    );
    content.append(&entry);

    let open_button = gtk::Button::builder()
        .label(gettext("Open Login Page in Browser"))
        .build();
    open_button.connect_clicked(|_| open_in_browser());
    content.append(&open_button);

    let login_button = gtk::Button::builder()
        .label(gettext("Login"))
        .css_classes(["suggested-action"])
        .build();
    content.append(&login_button);

    let dialog = adw::Window::builder()
        .title(gettext("Web Login"))
        .default_width(460)
        .modal(true)
        .content(&content)
        .build();
    dialog.set_transient_for(Some(parent));

    let dialog_weak = dialog.downgrade();
    let sender_clone = sender.clone();
    let entry_clone = entry.clone();
    login_button.connect_clicked(move |_| {
        let text = entry_clone.text().to_string();
        if login_with_cookies(&sender_clone, &text) {
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.close();
            }
        } else {
            entry_clone.add_css_class("error");
        }
    });

    dialog.present();
}

/// 内嵌官方登录页，登录成功后自动取回 cookie
#[cfg(feature = "webview")]
fn show_webview_window(parent: &gtk::Window, sender: Sender<Action>) {
    use webkit6::prelude::*;

    let webview = webkit6::WebView::builder()
        .hexpand(true)
        .vexpand(true)
        .build();
    if let Some(settings) = WebViewExt::settings(&webview) {
        settings.set_enable_developer_extras(false);
        settings.set_user_agent(Some(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/120.0.0.0 Safari/537.36",
        ));
    }

    let hint = gtk::Label::builder()
        .label(gettext("You will be signed in automatically once the web page logs in."))
        .css_classes(["caption", "dim-label"])
        .margin_top(6)
        .margin_bottom(6)
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&webview);
    content.append(&hint);

    // 页面渲染异常时的第二条兜底：外部浏览器登录后粘贴 cookie
    let header = adw::HeaderBar::new();
    let browser_button = gtk::Button::builder()
        .label(gettext("Open in Browser"))
        .css_classes(["flat"])
        .build();
    browser_button.connect_clicked(|_| open_in_browser());
    let paste_button = gtk::Button::builder()
        .label(gettext("Paste Cookie"))
        .css_classes(["flat"])
        .build();
    header.pack_start(&browser_button);
    header.pack_start(&paste_button);

    // AdwWindow 不支持 set_titlebar，用 AdwToolbarView 承载标题栏
    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&content));

    let dialog = adw::Window::builder()
        .title(gettext("Web Login"))
        .default_width(520)
        .default_height(680)
        .content(&toolbar_view)
        .build();
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);

    let sender_for_paste = sender.clone();
    let dialog_weak_for_paste = dialog.downgrade();
    paste_button.connect_clicked(move |_| {
        if let Some(dialog) = dialog_weak_for_paste.upgrade() {
            show_cookie_dialog(dialog.upcast_ref::<gtk::Window>(), sender_for_paste.clone());
        }
    });

    webview.load_uri(LOGIN_URL);


    // 轮询页面 cookie，出现 MUSIC_U 即视为登录成功
    // WebKitGTK 6 起 cookie 归属 NetworkSession
    let cookie_manager = WebViewExt::network_session(&webview)
        .and_then(|session| session.cookie_manager());
    let dialog_weak = dialog.downgrade();
    let polling = std::rc::Rc::new(std::cell::Cell::new(true));
    let polling_clone = polling.clone();

    glib::timeout_add_seconds_local(1, move || {
        if !polling_clone.get() {
            return glib::ControlFlow::Break;
        }
        let Some(cookie_manager) = cookie_manager.clone() else {
            return glib::ControlFlow::Break;
        };
        let sender = sender.clone();
        let dialog_weak = dialog_weak.clone();
        let polling = polling_clone.clone();
        glib::spawn_future_local(async move {
            let cookies = match cookie_manager.all_cookies_future().await {
                Ok(cookies) => cookies,
                Err(err) => {
                    debug!("读取页面 cookie 失败: {:?}", err);
                    return;
                }
            };
            let picked: Vec<(String, String)> = cookies
                .into_iter()
                .filter_map(|mut c| {
                    let name = c.name()?.to_string();
                    let value = c.value()?.to_string();
                    WANTED_COOKIES
                        .contains(&name.as_str())
                        .then_some((name, value))
                })
                .collect();
            if !picked
                .iter()
                .any(|(name, value)| name == "MUSIC_U" && !value.is_empty())
            {
                return;
            }
            let names: Vec<&str> = picked.iter().map(|(n, _)| n.as_str()).collect();
            debug!("网页登录成功，取回 cookie: {:?}", names);
            polling.set(false);
            let cookie_jar = NcmClient::cookie_jar_from_pairs(picked);
            let _ = sender
                .send(Action::CheckLogin(UserMenuChild::Qr, cookie_jar))
                .await;
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.close();
            }
        });
        glib::ControlFlow::Continue
    });

    dialog.present();
}
