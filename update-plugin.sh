cargo build --release --bin trigger-message
spin plugin uninstall trigger-message
spin pluginify
spin plugin install -f trigger-message.json -y