#!/bin/bash
set -e

SERVICE_NAME="otternel"
BINARY_FILENAME="Otternel"
PROJECT_DIR="/opt/otternel"
BIN_PATH="/usr/local/bin/otternel"

echo "Starting Otternel deployment"

cd "$PROJECT_DIR"

echo "Pulling latest changes..."
git pull origin main

echo "Compiling..."
cargo build --release

echo "Stopping systemd service..."
sudo systemctl stop "$SERVICE_NAME"

echo "Moving binary file..."
sudo cp "$PROJECT_DIR/target/release/$BINARY_FILENAME" "$BIN_PATH"
sudo chmod +x "$BIN_PATH"

echo "Restarting Otternel service..."
sudo systemctl daemon-reload
sudo systemctl start "$SERVICE_NAME"

echo "Checking service status :"
sudo systemctl status "$SERVICE_NAME" --no-pager

echo "End of Otternel deployment"
