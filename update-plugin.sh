cd ./trigger-message
cargo build --release
spin plugin uninstall trigger-message
spin pluginify
spin plugin install -f trigger-message.json -y