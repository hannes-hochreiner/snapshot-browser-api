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
  ^cargo update

  let deps_info = (get-deps-info)
  {
    "deps": ($deps_info.hash),
		"cargo_config": ($deps_info.cargo_config)
    "cargo_lock": (open Cargo.lock | hash sha256)
  } | to toml | save -f hashes.toml
  ^nix flake update
}

def get-deps-info [] {
  let temp_path_vendor = $"/tmp/snapshot_browser_deps_(random uuid)"
  let temp_path_home = $"/tmp/snapshot_browser_deps_(random uuid)"

  mkdir $temp_path_vendor
  mkdir $temp_path_home

	let deps_info = {
		cargo_config: (CARGO_HOME=$temp_path_home cargo vendor $temp_path_vendor --locked)
		hash: (nix hash path $temp_path_vendor)
	}

  rm -r $temp_path_home
  rm -r $temp_path_vendor

  $deps_info
}

export def create-test-container [] {
  sudo nixos-container create sb-api-test --flake .#sb-api-test
  sudo nixos-container start sb-api-test
}

export def update-test-container [] {
  sudo nixos-container update sb-api-test --flake .#sb-api-test
}

export def destroy-test-container [] {
  let messed_up_path = "/var/lib/nixos-containers/sb-api-test/var/empty"
  
  if ($messed_up_path | path exists) {
    sudo chattr -i $messed_up_path
    # sudo rm -rf $messed_up_path
  }
  
  sudo nixos-container destroy sb-api-test
}

export def start [] {
  RUST_LOG=debug SNAPSHOT_CONFIG_PATH=./test_config.json ROCKET_ADDRESS=127.0.0.1 ROCKET_PORT=8080 ^cargo run
}