//! A folder can hold two entries of one name: the folder `Week/` and the menu
//! `Week.menu`, or the recipe `Pasta.cook` and the menu `Pasta.menu`.
//!
//! `cooklang_find::build_tree` used to key both by `Week` (or `Pasta`): the
//! folder's recipes were filed under the menu and vanished from the listing
//! and the static site, and one of the two `Pasta` was lost. The tree now keys
//! the second by its file name; the server links it with its extension, so
//! `/recipe/Pasta.menu` opens the menu, and the static site writes a page for
//! each.

#![cfg(feature = "server")]

#[path = "common/mod.rs"]
mod common;

use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

/// Kills the spawned server when the test ends, pass or panic.
struct ServerGuard {
    child: Child,
    port: u16,
    _dir: TempDir,
}

impl ServerGuard {
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }

    async fn page(&self, path: &str) -> String {
        let resp = reqwest::get(self.url(path)).await.expect("request");
        assert!(resp.status().is_success(), "{path}: {}", resp.status());
        resp.text().await.unwrap()
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("local addr").port()
}

/// `Pasta.cook` beside `Pasta.menu`, and `Plans/Week.menu` beside the folder
/// `Plans/Week/` that holds `Monday.menu`.
fn write_fixture(root: &Path) {
    std::fs::create_dir_all(root.join("Plans/Week")).unwrap();
    std::fs::write(root.join("Pasta.cook"), "Boil @pasta{200%g}.\n").unwrap();
    std::fs::write(
        root.join("Pasta.menu"),
        "== Day 1 ==\n\nLunch:\n- @./Pasta{}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("Plans/Week.menu"),
        "== Day 1 ==\n\nDinner:\n- @../Pasta{}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("Plans/Week/Monday.menu"),
        "== Day 1 ==\n\nBreakfast:\n- @../../Pasta{}\n",
    )
    .unwrap();
}

async fn start_server() -> ServerGuard {
    for _ in 0..5 {
        if let Some(server) = try_start_server().await {
            return server;
        }
    }
    panic!("could not start cook server on a free port after 5 attempts");
}

async fn try_start_server() -> Option<ServerGuard> {
    let dir = TempDir::new().expect("temp dir");
    let recipes = dir.path().join("recipes");
    write_fixture(&recipes_dir(&recipes));

    let port = free_port();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.arg("server")
        .arg(&recipes)
        .arg("--port")
        .arg(port.to_string());
    let child = common::with_isolated_config(&mut cmd, dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cook server");

    let mut guard = ServerGuard {
        child,
        port,
        _dir: dir,
    };

    let client = reqwest::Client::new();
    let url = guard.url("/api/recipes");
    for _ in 0..200 {
        if guard.child.try_wait().expect("poll server").is_some() {
            // Port was taken between reserving and binding it.
            return None;
        }
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return Some(guard);
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("cook server on port {port} never became ready");
}

fn recipes_dir(path: &Path) -> std::path::PathBuf {
    std::fs::create_dir_all(path).unwrap();
    path.to_path_buf()
}

#[tokio::test]
async fn the_listing_links_both_entries_of_a_shared_name() {
    let server = start_server().await;

    let root = server.page("/").await;
    assert!(root.contains(r#"href="/recipe/Pasta""#), "{root}");
    assert!(root.contains(r#"href="/recipe/Pasta.menu""#), "{root}");
    // Each card is named `Pasta`, not after the tree's key.
    assert!(!root.contains(r#"data-name="Pasta.menu""#), "{root}");

    let plans = server.page("/directory/Plans").await;
    assert!(plans.contains(r#"href="/directory/Plans/Week""#), "{plans}");
    assert!(plans.contains(r#"href="/recipe/Plans/Week"#), "{plans}");

    let week = server.page("/directory/Plans/Week").await;
    assert!(
        week.contains(r#"href="/recipe/Plans/Week/Monday""#),
        "{week}"
    );

    // Each link opens its own file.
    assert!(server.page("/recipe/Pasta").await.contains("Boil"));
    let menu = server.page("/recipe/Pasta.menu").await;
    assert!(menu.contains("Lunch"), "{menu}");
    // Its last breadcrumb is the menu's name, not the file's.
    assert!(
        menu.contains(r#"<span class="text-text">Pasta</span>"#),
        "{menu}"
    );
    assert!(!menu.contains("Pasta.menu</span>"), "{menu}");
}

#[test]
fn the_static_site_writes_a_page_for_both_entries_of_a_shared_name() {
    let dir = TempDir::new().unwrap();
    let recipes = recipes_dir(&dir.path().join("recipes"));
    write_fixture(&recipes);
    let out = dir.path().join("_site");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("cook"));
    cmd.args(["build", "web"])
        .arg(&out)
        .arg("--base-path")
        .arg(&recipes);
    let status = common::with_isolated_config(&mut cmd, dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run cook build web");
    assert!(status.success());

    let read = |page: &str| {
        std::fs::read_to_string(out.join(page)).unwrap_or_else(|e| panic!("{page}: {e}"))
    };
    assert!(read("recipe/Pasta.html").contains("Boil"));
    assert!(read("menu/Pasta.html").contains("Lunch"));
    assert!(read("menu/Plans/Week.html").contains("Dinner"));
    assert!(read("menu/Plans/Week/Monday.html").contains("Breakfast"));
    assert!(read("directory/Plans/Week.html").contains("Monday"));
}
