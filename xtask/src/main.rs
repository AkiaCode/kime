use is_executable::is_executable;
use std::path::PathBuf;
use std::process::Command;
use std::{collections::HashMap, path::Path};
use structopt::StructOpt;
use strum::IntoEnumIterator;

trait CommandExt: Sized {
    fn mode(&mut self, mode: BuildMode) -> &mut Self;
}

impl CommandExt for Command {
    fn mode(&mut self, mode: BuildMode) -> &mut Self {
        if mode.is_release() {
            self.arg("--release")
        } else {
            self
        }
    }
}

#[derive(Clone, Copy, strum_macros::EnumString)]
enum BuildMode {
    Debug,
    Release,
}

impl BuildMode {
    fn cmake_build_type(self) -> &'static str {
        match self {
            BuildMode::Debug => "-DCMAKE_BUILD_TYPE=Debug",
            BuildMode::Release => "-DCMAKE_BUILD_TYPE=Release",
        }
    }

    fn is_release(self) -> bool {
        match self {
            BuildMode::Debug => false,
            BuildMode::Release => true,
        }
    }

    fn cargo_target_dir(self) -> &'static str {
        match self {
            BuildMode::Debug => "target/debug",
            BuildMode::Release => "target/release",
        }
    }
}

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    strum_macros::EnumString,
    strum_macros::Display,
    strum_macros::EnumIter,
)]
enum Frontend {
    #[strum(to_string = "XIM")]
    Xim,
    #[strum(to_string = "WAYLAND")]
    Wayland,
    #[strum(to_string = "QT5")]
    Qt5,
    #[strum(to_string = "QT6")]
    Qt6,
    #[strum(to_string = "GTK2")]
    Gtk2,
    #[strum(to_string = "GTK3")]
    Gtk3,
    #[strum(to_string = "GTK4")]
    Gtk4,
}

impl Frontend {
    pub fn is_cmake(self) -> bool {
        match self {
            Frontend::Xim | Frontend::Wayland => false,
            _ => true,
        }
    }
}

#[derive(StructOpt)]
enum TaskCommand {
    Test,
    Clean,
    Build {
        #[structopt(long, parse(try_from_str), default_value = "Release")]
        mode: BuildMode,
        #[structopt(
            parse(try_from_str),
            about = "Select frontend availiable list: [XIM, WAYLAND, QT5, QT6, GTK2, GTK3, GTK4]"
        )]
        frontends: Vec<Frontend>,
    },
    Install {
        #[structopt(parse(from_os_str))]
        target_path: PathBuf,
    },
    ReleaseDeb {
        #[structopt(parse(from_os_str))]
        target_path: Option<PathBuf>,
    },
}

impl TaskCommand {
    pub fn run(self) {
        match self {
            TaskCommand::ReleaseDeb { target_path } => {
                let target_path = target_path
                    .unwrap_or_else(|| std::env::current_dir().expect("Get current dir"));
                let deb_dir = tempfile::tempdir().expect("Create tempdir");
                let control_path = deb_dir.as_ref().join("DEBIAN/control");
                std::fs::create_dir_all(control_path.parent().unwrap()).expect("Create DEBIAN dir");

                std::fs::write(
                    control_path,
                    std::fs::read_to_string(get_src_path().join("xtask").join("control.in"))
                        .expect("Read control.in")
                        .replace("%VER%", env!("CARGO_PKG_VERSION")),
                )
                .expect("Write control");

                // Install into tempdir
                TaskCommand::Install {
                    target_path: deb_dir.path().into(),
                }
                .run();

                Command::new("dpkg-deb")
                    .arg("--build")
                    .arg(deb_dir.as_ref())
                    .arg(target_path.join(format!("kime_{}_amd64.deb", env!("CARGO_PKG_VERSION"))))
                    .spawn()
                    .expect("Spawn dpkg-deb")
                    .wait()
                    .expect("Run dpkg-deb");
            }
            TaskCommand::Install { target_path } => {
                let out_path = get_build_path().join("out");

                install(
                    out_path.join("kime-xim"),
                    target_path.join("usr/bin/kime-xim"),
                );
                install(
                    out_path.join("kime-wayland"),
                    target_path.join("usr/bin/kime-wayland"),
                );
                install(
                    out_path.join("libkime-gtk2.so"),
                    target_path.join("usr/lib/gtk-2.0/2.10.0/immodules/im-kime.so"),
                );
                install(
                    out_path.join("libkime-gtk3.so"),
                    target_path.join("usr/lib/gtk-3.0/3.0.0/immodules/im-kime.so"),
                );
                install(
                    out_path.join("libkime-gtk4.so"),
                    target_path.join("usr/lib/gtk-4.0/4.0.0/immodules/libkime-gtk4.so"),
                );
                install(out_path.join("libkime-qt5.so"), target_path.join("usr/lib/qt/plugins/platforminputcontexts/libkimeplatforminputcontextplugin.so"));
                install(out_path.join("libkime-qt6.so"), target_path.join("usr/lib/qt6/plugins/platforminputcontexts/libkimeplatforminputcontextplugin.so"));
                install(
                    out_path.join("libkime_engine.so"),
                    target_path.join("usr/lib/libkime_engine.so"),
                );
                install(
                    out_path.join("kime_engine.h"),
                    target_path.join("usr/include/kime_engine.h"),
                );
                install(
                    out_path.join("config.yaml"),
                    target_path.join("etc/kime/config.yaml"),
                );
            }
            TaskCommand::Test => {
                Command::new("cargo")
                    .args(&["test", "-p=kime-engine-core"])
                    .current_dir(get_src_path())
                    .spawn()
                    .expect("Spawn cargo")
                    .wait()
                    .expect("Run test");
            }
            TaskCommand::Clean => {}
            TaskCommand::Build { frontends, mode } => {
                let mut frontends = frontends
                    .into_iter()
                    .map(|f| (f, true))
                    .collect::<HashMap<_, _>>();

                for f in Frontend::iter() {
                    frontends.entry(f).or_insert(false);
                }

                let src_path = get_src_path();
                let build_path = get_build_path();
                let out_path = build_path.join("out");
                let cmake_path = build_path.join("cmake");
                let cmake_out_path = cmake_path.join("lib");

                std::fs::create_dir_all(&out_path).expect("create out_path");
                std::fs::create_dir_all(&cmake_out_path).expect("create cmake_out_path");

                // build engine core
                build_core(mode);

                std::fs::copy(
                    src_path
                        .join(mode.cargo_target_dir())
                        .join("libkime_engine.so"),
                    out_path.join("libkime_engine.so"),
                )
                .expect("Copy engine file");

                if frontends[&Frontend::Xim] {
                    Command::new("cargo")
                        .args(&["build", "--bin=kime-xim"])
                        .current_dir(get_src_path())
                        .mode(mode)
                        .spawn()
                        .expect("Spawn cargo")
                        .wait()
                        .expect("Run cargo");

                    std::fs::copy(
                        src_path.join(mode.cargo_target_dir()).join("kime-xim"),
                        out_path.join("kime-xim"),
                    )
                    .expect("Copy xim file");
                }

                if frontends[&Frontend::Wayland] {
                    Command::new("cargo")
                        .args(&["build", "--bin=kime-wayland"])
                        .current_dir(get_src_path())
                        .mode(mode)
                        .spawn()
                        .expect("Spawn cargo")
                        .wait()
                        .expect("Run cargo");

                    std::fs::copy(
                        src_path.join(mode.cargo_target_dir()).join("kime-wayland"),
                        out_path.join("kime-wayland"),
                    )
                    .expect("Copy xim file");
                }

                let mut cmake_command = Command::new("cmake");

                cmake_command
                    .current_dir(&cmake_path)
                    .arg(&src_path)
                    .arg("-GNinja")
                    .arg(mode.cmake_build_type());

                for (frontend, on) in frontends.iter() {
                    if !frontend.is_cmake() {
                        continue;
                    }
                    let flag = if *on { "ON" } else { "OFF" };

                    cmake_command.arg(format!("-DENABLE_{}={}", frontend, flag));
                }

                cmake_command
                    .spawn()
                    .expect("Spawn cmake")
                    .wait()
                    .expect("Run cmake");

                Command::new("ninja")
                    .current_dir(&cmake_path)
                    .spawn()
                    .expect("Spawn ninja")
                    .wait()
                    .expect("Run ninja");

                for file in cmake_out_path.read_dir().expect("Read cmake out") {
                    let file = file.expect("Read entry");

                    std::fs::copy(file.path(), &out_path.join(file.file_name()))
                        .expect("Copy file");
                }

                std::fs::copy(
                    src_path.join("engine").join("cffi").join("kime_engine.h"),
                    out_path.join("kime_engine.h"),
                )
                .expect("Copy engine header file");

                std::fs::copy(
                    src_path.join("docs").join("default_config.yaml"),
                    out_path.join("config.yaml"),
                )
                .expect("Copy default config file");

                if mode.is_release() {
                    strip_all(&out_path).ok();
                }
            }
        }
    }
}

fn get_src_path() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn get_build_path() -> PathBuf {
    get_src_path().join("build")
}

fn build_core(mode: BuildMode) {
    Command::new("cargo")
        .args(&["build", "-p=kime-engine-capi"])
        .mode(mode)
        .spawn()
        .expect("Spawn cargo")
        .wait()
        .expect("Run cargo");
}

fn strip_all(dir: &Path) -> std::io::Result<()> {
    for path in dir.read_dir()? {
        let path = path?.path();

        if !is_executable(&path) {
            continue;
        }

        Command::new("strip")
            .arg("-s")
            .arg(path)
            .spawn()
            .expect("Spawn strip")
            .wait()
            .expect("Run strip");
    }

    Ok(())
}

fn install(src: PathBuf, target: PathBuf) {
    if src.exists() {
        println!("Install {} into {}", src.display(), target.display());

        Command::new("install")
            .arg(if is_executable(&src) {
                "-Dsm755"
            } else {
                "-Dm644"
            })
            .arg(src)
            .arg("-T")
            .arg(target)
            .spawn()
            .expect("Spawn install")
            .wait()
            .expect("Run install");
    }
}

fn main() {
    let args = TaskCommand::from_args();

    args.run();
}
