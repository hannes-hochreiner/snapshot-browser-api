export def build [] {
  test
	cargo build
}

export def test [] {
	cargo test
}

export def nix-build [] {
  ^nix build
}

export def update [] {
	let deps_info = (get-deps-info)

  ^cargo update
  {
    "deps": ($deps_info.hash),
		"cargo_config": ($deps_info.cargo_config)
    "cargo_lock": (open Cargo.lock | hash sha256)
  } | to toml | save -f hashes.toml
  ^nix flake update
}

def get-deps-info [] {
  let temp_path = $"/tmp/snapshot_browser_deps_(random uuid)"

  mkdir $temp_path
	let deps_info = {
		cargo_config: (cargo vendor $temp_path)
		hash: (nix hash path $temp_path)
	}

  rm -r $temp_path

  $deps_info
}
