import threading
import time

import requests

import serena_core


def main():
    port = 24283
    print(f"Starting Rust backend on port {port}...")

    # Run server in a thread because start_rust_backend blocks
    t = threading.Thread(target=serena_core.start_rust_backend, args=(port,))
    t.daemon = True
    t.start()

    # Wait for startup
    time.sleep(1)

    try:
        response = requests.get(f"http://127.0.0.1:{port}/heartbeat")
        print(f"Heartbeat response: {response.json()}")
    except Exception as e:
        print(f"Failed to connect: {e}")


if __name__ == "__main__":
    main()
