#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Alfred Jean LLC

"""Minimal MCP server for integration testing.

Implements the MCP protocol for testing:
- initialize: Returns server info and capabilities
- notifications/initialized: Acknowledges initialization
- tools/list: Returns one echo tool
- tools/call: Echoes back the input arguments

Usage:
    python3 echo_mcp_server.py

The server reads JSON-RPC requests from stdin (one per line)
and writes JSON-RPC responses to stdout.
"""
import json
import sys


def main():
    for line in sys.stdin:
        try:
            req = json.loads(line.strip())
            method = req.get("method", "")
            req_id = req.get("id")
            params = req.get("params", {})

            if method == "initialize":
                result = {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {"listChanged": False}},
                    "serverInfo": {"name": "echo-test", "version": "1.0.0"},
                }
            elif method == "notifications/initialized":
                # Notifications don't get responses
                continue
            elif method == "tools/list":
                result = {
                    "tools": [
                        {
                            "name": "echo",
                            "description": "Echo back input arguments",
                            "inputSchema": {
                                "type": "object",
                                "properties": {"message": {"type": "string"}},
                            },
                        },
                        {
                            "name": "fail",
                            "description": "Always returns an error",
                            "inputSchema": {"type": "object"},
                        },
                    ]
                }
            elif method == "tools/call":
                tool_name = params.get("name", "")
                arguments = params.get("arguments", {})

                if tool_name == "echo":
                    result = {
                        "content": [{"type": "text", "text": json.dumps(arguments)}],
                        "isError": False,
                    }
                elif tool_name == "fail":
                    result = {
                        "content": [{"type": "text", "text": "Intentional failure"}],
                        "isError": True,
                    }
                else:
                    # Unknown tool
                    resp = {
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "error": {
                            "code": -32601,
                            "message": f"Tool not found: {tool_name}",
                        },
                    }
                    print(json.dumps(resp), flush=True)
                    continue
            else:
                # Unknown method
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {"code": -32601, "message": f"Method not found: {method}"},
                }
                print(json.dumps(resp), flush=True)
                continue

            if req_id is not None:
                resp = {"jsonrpc": "2.0", "id": req_id, "result": result}
                print(json.dumps(resp), flush=True)

        except json.JSONDecodeError as e:
            if "req_id" in locals() and req_id is not None:
                err = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {"code": -32700, "message": f"Parse error: {e}"},
                }
                print(json.dumps(err), flush=True)
        except Exception as e:
            if "req_id" in locals() and req_id is not None:
                err = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {"code": -32603, "message": str(e)},
                }
                print(json.dumps(err), flush=True)


if __name__ == "__main__":
    main()
