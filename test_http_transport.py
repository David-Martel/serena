#!/usr/bin/env python3
"""Test script for Serena HTTP MCP transport"""

import json
import requests
import sys

BASE_URL = "http://localhost:3000"

def test_health_check():
    """Test the health check endpoint"""
    print("Testing health check endpoint...")
    try:
        response = requests.get(f"{BASE_URL}/health")
        response.raise_for_status()
        data = response.json()
        print(f"✓ Health check passed: {data}")
        return True
    except Exception as e:
        print(f"✗ Health check failed: {e}")
        return False

def test_initialize():
    """Test the MCP initialize method"""
    print("\nTesting MCP initialize method...")
    try:
        request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }
        response = requests.post(
            f"{BASE_URL}/mcp",
            json=request,
            headers={"Content-Type": "application/json"}
        )
        response.raise_for_status()
        data = response.json()

        if "error" in data:
            print(f"✗ Initialize failed with error: {data['error']}")
            return False

        print(f"✓ Initialize succeeded:")
        print(f"  Protocol version: {data['result']['protocolVersion']}")
        print(f"  Server: {data['result']['serverInfo']['name']} v{data['result']['serverInfo']['version']}")
        return True
    except Exception as e:
        print(f"✗ Initialize failed: {e}")
        return False

def test_ping():
    """Test the MCP ping method"""
    print("\nTesting MCP ping method...")
    try:
        request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "ping",
            "params": {}
        }
        response = requests.post(
            f"{BASE_URL}/mcp",
            json=request,
            headers={"Content-Type": "application/json"}
        )
        response.raise_for_status()
        data = response.json()

        if "error" in data:
            print(f"✗ Ping failed with error: {data['error']}")
            return False

        print(f"✓ Ping succeeded: {data}")
        return True
    except Exception as e:
        print(f"✗ Ping failed: {e}")
        return False

def test_list_tools():
    """Test the MCP tools/list method"""
    print("\nTesting MCP tools/list method...")
    try:
        request = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list",
            "params": {}
        }
        response = requests.post(
            f"{BASE_URL}/mcp",
            json=request,
            headers={"Content-Type": "application/json"}
        )
        response.raise_for_status()
        data = response.json()

        if "error" in data:
            print(f"✗ List tools failed with error: {data['error']}")
            return False

        tools = data['result'].get('tools', [])
        print(f"✓ List tools succeeded: {len(tools)} tools available")
        for tool in tools:
            print(f"  - {tool['name']}: {tool['description']}")
        return True
    except Exception as e:
        print(f"✗ List tools failed: {e}")
        return False

def test_batch_request():
    """Test the batch request endpoint"""
    print("\nTesting MCP batch request...")
    try:
        requests_batch = [
            {
                "jsonrpc": "2.0",
                "id": 10,
                "method": "ping",
                "params": {}
            },
            {
                "jsonrpc": "2.0",
                "id": 11,
                "method": "tools/list",
                "params": {}
            }
        ]
        response = requests.post(
            f"{BASE_URL}/mcp/batch",
            json=requests_batch,
            headers={"Content-Type": "application/json"}
        )
        response.raise_for_status()
        data = response.json()

        if isinstance(data, list) and len(data) == 2:
            print(f"✓ Batch request succeeded: {len(data)} responses")
            for item in data:
                if "error" in item:
                    print(f"  Request {item['id']}: ERROR - {item['error']['message']}")
                else:
                    print(f"  Request {item['id']}: SUCCESS")
            return True
        else:
            print(f"✗ Batch request failed: unexpected response format")
            return False
    except Exception as e:
        print(f"✗ Batch request failed: {e}")
        return False

def test_unknown_method():
    """Test handling of unknown methods"""
    print("\nTesting unknown method handling...")
    try:
        request = {
            "jsonrpc": "2.0",
            "id": 99,
            "method": "unknown_method_xyz",
            "params": {}
        }
        response = requests.post(
            f"{BASE_URL}/mcp",
            json=request,
            headers={"Content-Type": "application/json"}
        )
        response.raise_for_status()
        data = response.json()

        if "error" in data and data["error"]["code"] == -32601:
            print(f"✓ Unknown method properly rejected: {data['error']['message']}")
            return True
        else:
            print(f"✗ Unknown method not properly handled")
            return False
    except Exception as e:
        print(f"✗ Unknown method test failed: {e}")
        return False

def main():
    """Run all tests"""
    print("=" * 60)
    print("Serena MCP HTTP Transport Test Suite")
    print("=" * 60)

    tests = [
        test_health_check,
        test_initialize,
        test_ping,
        test_list_tools,
        test_batch_request,
        test_unknown_method,
    ]

    results = []
    for test in tests:
        results.append(test())

    print("\n" + "=" * 60)
    passed = sum(results)
    total = len(results)
    print(f"Results: {passed}/{total} tests passed")
    print("=" * 60)

    return 0 if all(results) else 1

if __name__ == "__main__":
    sys.exit(main())
