#!/bin/bash
#
# Install script for Actor Registry systemd services
#
# THIS SOFTWARE IS OPEN SOURCE UNDER THE MIT LICENSE
# Copyright 2025 Vincent Maciejewski, & M2 Tech
#
# Usage:
#   ./install.sh                    # Install GlobalRegistry service
#   ./install.sh manager mymanager  # Install a manager service
#   ./install.sh uninstall          # Remove all actor services
#
# Prerequisites:
#   - Root access (sudo)
#   - systemd
#   - 'actors' user exists (or specify different user)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SYSTEMD_DIR="/etc/systemd/system"
CONFIG_DIR="/etc/actors"
OPT_DIR="/opt/actors"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root (use sudo)"
        exit 1
    fi
}

create_user() {
    if ! id "actors" &>/dev/null; then
        log_info "Creating 'actors' user..."
        useradd --system --no-create-home --shell /sbin/nologin actors
    else
        log_info "User 'actors' already exists"
    fi
}

create_directories() {
    log_info "Creating directories..."
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$OPT_DIR/registry"
    mkdir -p "$OPT_DIR/managers"
    chown -R actors:actors "$OPT_DIR"
}

install_registry() {
    log_info "Installing GlobalRegistry service..."

    # Copy service file
    cp "$SCRIPT_DIR/global-registry.service" "$SYSTEMD_DIR/"

    # Copy Python package if not already installed
    if [[ -d "$SCRIPT_DIR/../python/actors_registry" ]]; then
        log_info "Copying Python registry package..."
        cp -r "$SCRIPT_DIR/../python/actors_registry" "$OPT_DIR/registry/"
    fi

    # Create default config if not exists
    if [[ ! -f "$CONFIG_DIR/registry.json" ]]; then
        log_info "Creating default registry config..."
        cat > "$CONFIG_DIR/registry.json" << 'EOF'
{
  "registry_endpoint": "tcp://0.0.0.0:5555",
  "heartbeat_timeout_s": 6.0,
  "heartbeat_check_interval_s": 1.0,
  "hosts": {}
}
EOF
    fi

    systemctl daemon-reload
    log_info "GlobalRegistry service installed"
    log_info "To enable: sudo systemctl enable global-registry"
    log_info "To start:  sudo systemctl start global-registry"
}

install_manager() {
    local manager_name="$1"

    if [[ -z "$manager_name" ]]; then
        log_error "Manager name required"
        echo "Usage: $0 manager <name>"
        exit 1
    fi

    log_info "Installing manager service: $manager_name"

    # Create manager directory
    mkdir -p "$OPT_DIR/managers/$manager_name"
    chown -R actors:actors "$OPT_DIR/managers/$manager_name"

    # Create service file from template
    local service_file="$SYSTEMD_DIR/manager-$manager_name.service"
    sed "s/%i/$manager_name/g" "$SCRIPT_DIR/manager-template.service" > "$service_file"

    systemctl daemon-reload
    log_info "Manager service installed: manager-$manager_name"
    log_info "To enable: sudo systemctl enable manager-$manager_name"
    log_info "To start:  sudo systemctl start manager-$manager_name"
    log_info ""
    log_info "Next steps:"
    log_info "1. Copy your manager binary to: $OPT_DIR/managers/$manager_name/"
    log_info "2. Edit service file if needed: $service_file"
    log_info "3. Enable and start the service"
}

uninstall() {
    log_info "Uninstalling actor services..."

    # Stop and disable services
    for service in "$SYSTEMD_DIR"/manager-*.service "$SYSTEMD_DIR/global-registry.service"; do
        if [[ -f "$service" ]]; then
            local name=$(basename "$service")
            log_info "Stopping $name..."
            systemctl stop "$name" 2>/dev/null || true
            systemctl disable "$name" 2>/dev/null || true
            rm -f "$service"
        fi
    done

    systemctl daemon-reload
    log_info "Services removed"
    log_info "Note: $OPT_DIR and $CONFIG_DIR were NOT removed"
}

show_status() {
    echo "=== Actor Services Status ==="
    echo ""

    if systemctl is-active --quiet global-registry 2>/dev/null; then
        echo -e "GlobalRegistry: ${GREEN}running${NC}"
    else
        echo -e "GlobalRegistry: ${YELLOW}stopped${NC}"
    fi

    for service in "$SYSTEMD_DIR"/manager-*.service; do
        if [[ -f "$service" ]]; then
            local name=$(basename "$service" .service)
            if systemctl is-active --quiet "$name" 2>/dev/null; then
                echo -e "$name: ${GREEN}running${NC}"
            else
                echo -e "$name: ${YELLOW}stopped${NC}"
            fi
        fi
    done
}

show_usage() {
    echo "Usage: $0 [command] [args]"
    echo ""
    echo "Commands:"
    echo "  (none)              Install GlobalRegistry service"
    echo "  manager <name>      Install a manager service"
    echo "  uninstall           Remove all actor services"
    echo "  status              Show service status"
    echo "  help                Show this help"
    echo ""
    echo "Examples:"
    echo "  sudo $0                     # Install GlobalRegistry"
    echo "  sudo $0 manager pricer      # Install manager-pricer service"
    echo "  sudo $0 manager risk        # Install manager-risk service"
    echo "  sudo $0 status              # Check all services"
    echo "  sudo $0 uninstall           # Remove all services"
}

# Main
case "${1:-}" in
    "")
        check_root
        create_user
        create_directories
        install_registry
        ;;
    manager)
        check_root
        create_user
        create_directories
        install_manager "$2"
        ;;
    uninstall)
        check_root
        uninstall
        ;;
    status)
        show_status
        ;;
    help|--help|-h)
        show_usage
        ;;
    *)
        log_error "Unknown command: $1"
        show_usage
        exit 1
        ;;
esac
