from pathlib import Path

import serena_core

# Define resources to fetch for Windows (amd64)
resources = [
    {
        "name": "taplo",
        "url": "https://github.com/tamasfe/taplo/releases/download/0.9.3/taplo-full-windows-x86_64.zip",
        "executable": "taplo.exe",
    },
    {
        "name": "lua-language-server",
        "url": "https://github.com/LuaLS/lua-language-server/releases/download/3.13.6/lua-language-server-3.13.6-win32-x64.zip",
        "executable": "bin/lua-language-server.exe",
    },
    {
        "name": "marksman",
        "url": "https://github.com/artempyanykh/marksman/releases/download/2024-12-18/marksman.exe",
        "executable": "marksman.exe",
    },
    {
        "name": "terraform-ls",
        "url": "https://releases.hashicorp.com/terraform-ls/0.36.5/terraform-ls_0.36.5_windows_amd64.zip",
        "executable": "terraform-ls.exe",
    },
    {
        "name": "clojure-lsp",
        "url": "https://github.com/clojure-lsp/clojure-lsp/releases/latest/download/clojure-lsp-native-windows-amd64.zip",
        "executable": "clojure-lsp.exe",
    },
]


def main():
    root_dir = Path.home() / ".serena" / "ls_resources"
    print(f"Downloading resources to {root_dir} using Rust core...")

    for res in resources:
        print(f"Fetching {res['name']}...")
        try:
            path = serena_core.ensure_tool(res["name"], res["url"], res["executable"], str(root_dir))
            print(f"✅ {res['name']} ready at {path}")
        except Exception as e:
            print(f"❌ Failed to fetch {res['name']}: {e}")


if __name__ == "__main__":
    main()
