use std::{env, path::PathBuf, process::Command};

fn main() {
  let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
  let portal = root.join("portal");

  println!("cargo:rerun-if-changed=portal/index.html");
  println!("cargo:rerun-if-changed=portal/src");
  println!("cargo:rerun-if-changed=portal/plugins");
  println!("cargo:rerun-if-changed=portal/package.json");
  println!("cargo:rerun-if-changed=portal/package-lock.json");
  println!("cargo:rerun-if-changed=portal/tsconfig.json");
  println!("cargo:rerun-if-changed=portal/vite.config.ts");

  if env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("xtensa") {
    return;
  }

  // On Windows npm is npm.cmd
  let npm = if cfg!(target_os = "windows") {
    "npm.cmd"
  } else {
    "npm"
  };

  let install = Command::new(npm)
    .arg("ci")
    .current_dir(&portal)
    .status()
    .expect("npm ci failed — is Node.js installed?");
  assert!(install.success(), "npm ci exited with error");

  let build = Command::new(npm)
    .args(["run", "build"])
    .current_dir(&portal)
    .status()
    .expect("npm run build failed");
  assert!(build.success(), "Vite build exited with error");

  linker_be_nice();
  // make sure linkall.x is the last linker script (otherwise might cause problems
  // with flip-link)
  println!("cargo:rustc-link-arg=-Tlinkall.x");
}

fn linker_be_nice() {
  let args: Vec<String> = std::env::args().collect();
  if args.len() > 1 {
    let kind = &args[1];
    let what = &args[2];

    match kind.as_str() {
      "undefined-symbol" => match what.as_str() {
        what if what.starts_with("_defmt_") => {
          eprintln!();
          eprintln!(
            "💡 `defmt` not found - make sure `defmt.x` is added as a linker script and you have included `use defmt_rtt as _;`"
          );
          eprintln!();
        }
        "_stack_start" => {
          eprintln!();
          eprintln!("💡 Is the linker script `linkall.x` missing?");
          eprintln!();
        }
        what if what.starts_with("esp_rtos_") => {
          eprintln!();
          eprintln!(
            "💡 `esp-radio` has no scheduler enabled. Make sure you have initialized `esp-rtos` or provided an external scheduler."
          );
          eprintln!();
        }
        "embedded_test_linker_file_not_added_to_rustflags" => {
          eprintln!();
          eprintln!(
            "💡 `embedded-test` not found - make sure `embedded-test.x` is added as a linker script for tests"
          );
          eprintln!();
        }
        "free"
        | "malloc"
        | "calloc"
        | "get_free_internal_heap_size"
        | "malloc_internal"
        | "realloc_internal"
        | "calloc_internal"
        | "free_internal" => {
          eprintln!();
          eprintln!(
            "💡 Did you forget the `esp-alloc` dependency or didn't enable the `compat` feature on it?"
          );
          eprintln!();
        }
        _ => (),
      },
      // we don't have anything helpful for "missing-lib" yet
      _ => {
        std::process::exit(1);
      }
    }

    std::process::exit(0);
  }

  println!(
    "cargo:rustc-link-arg=-Wl,--error-handling-script={}",
    std::env::current_exe().unwrap().display()
  );
}
