#!/bin/sh
# install.sh -- Timeline DSL CLI installer
# Usage: curl -sSfL https://raw.githubusercontent.com/keroway/timeline-dsl/main/install.sh | sh
# Set TDSL_VERSION to install a specific version (default: latest)

set -e

REPO="keroway/timeline-dsl"
INSTALL_DIR="${HOME}/.local/bin"
BIN_NAME="tdsl"

# Detect OS and architecture
detect_platform() {
    os=$(uname -s)
    arch=$(uname -m)

    case "${os}" in
        Linux)
            case "${arch}" in
                x86_64)
                    echo "tdsl-linux-x86_64.tar.gz"
                    ;;
                aarch64|arm64)
                    echo "tdsl-linux-aarch64.tar.gz"
                    ;;
                *)
                    echo "Unsupported Linux architecture: ${arch}" >&2
                    exit 1
                    ;;
            esac
            ;;
        Darwin)
            case "${arch}" in
                x86_64)
                    echo "tdsl-macos-x86_64.tar.gz"
                    ;;
                arm64)
                    echo "tdsl-macos-aarch64.tar.gz"
                    ;;
                *)
                    echo "Unsupported macOS architecture: ${arch}" >&2
                    exit 1
                    ;;
            esac
            ;;
        *)
            echo "Unsupported OS: ${os}" >&2
            exit 1
            ;;
    esac
}

# Resolve version to install
resolve_version() {
    if [ -n "${TDSL_VERSION}" ]; then
        echo "${TDSL_VERSION}"
        return
    fi

    # Fetch latest release tag from GitHub API
    latest=$(curl -sSfL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' \
        | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

    if [ -z "${latest}" ]; then
        echo "Failed to fetch latest release version from GitHub." >&2
        exit 1
    fi

    echo "${latest}"
}

main() {
    archive=$(detect_platform)
    version=$(resolve_version)

    download_url="https://github.com/${REPO}/releases/download/${version}/${archive}"

    echo "Installing ${BIN_NAME} ${version} (${archive})..."

    # Create install directory if it does not exist
    mkdir -p "${INSTALL_DIR}"

    # Download and extract into a temporary directory
    tmp_dir=$(mktemp -d)
    trap 'rm -rf "${tmp_dir}"' EXIT

    echo "Downloading from ${download_url} ..."
    curl -sSfL "${download_url}" -o "${tmp_dir}/${archive}"

    tar xzf "${tmp_dir}/${archive}" -C "${tmp_dir}"

    # Install binary
    install -m 755 "${tmp_dir}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"

    echo "Installed ${BIN_NAME} to ${INSTALL_DIR}/${BIN_NAME}"

    # Warn if INSTALL_DIR is not in PATH
    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*)
            ;;
        *)
            echo ""
            echo "NOTE: ${INSTALL_DIR} is not in your PATH."
            echo "Add the following line to your shell profile (~/.bashrc, ~/.zshrc, ~/.profile, etc.):"
            echo ""
            echo "  export PATH=\"\${HOME}/.local/bin:\${PATH}\""
            echo ""
            ;;
    esac
}

main
