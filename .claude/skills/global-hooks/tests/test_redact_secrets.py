"""Tests for hooks/redact_secrets.py, run as Claude Code runs it: a process fed
a PostToolUse event on stdin. They need gitleaks on PATH, as the hook does.

The fake secrets are assembled at run time, so this file holds none for the
repository's own secret scans to find.
"""

import json
import os
import subprocess
import sys
import unittest

HOOK = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "hooks", "redact_secrets.py")
CHARS = "Q7vXk2mPz9RtL4wNc8HbJ5yFd3GsA6uE1oVi0KqTnWjYeZr"

FAKE = {
    "github-pat": "ghp_" + CHARS[:36],
    "google-oauth-access-token": "ya29." + CHARS[3:43],
    "gcp-client-secret": "GOCSPX-" + CHARS[5:33],
    "gcp-refresh-token": "1//0g" + CHARS[1:45],
    "rclone-obscured-password": CHARS[7:37],
}


def run_hook(event, **env):
    return subprocess.run([sys.executable, HOOK], input=json.dumps(event),
                          capture_output=True, text=True, timeout=60,
                          env={**os.environ, **env})


def bash_event(stdout, stderr=""):
    return {"hook_event_name": "PostToolUse", "tool_name": "Bash",
            "tool_input": {"command": "cat creds"},
            "tool_response": {"stdout": stdout, "stderr": stderr, "interrupted": False,
                              "isImage": False, "noOutputExpected": False}}


def read_event(content):
    return {"hook_event_name": "PostToolUse", "tool_name": "Read",
            "tool_input": {"file_path": "/tmp/creds.conf"},
            "tool_response": {"type": "text", "file": {
                "filePath": "/tmp/creds.conf", "content": content,
                "numLines": content.count("\n") + 1, "startLine": 1,
                "totalLines": content.count("\n") + 1}}}


def updated(result):
    return json.loads(result.stdout)["hookSpecificOutput"]["updatedToolOutput"]


class RedactSecrets(unittest.TestCase):
    def assert_redacted(self, text, *names):
        for name in names:
            self.assertNotIn(FAKE[name], text)
        self.assertIn("[REDACTED by gitleaks:", text)

    def test_bash_output_is_redacted_and_keeps_its_shape(self):
        event = bash_event(
            stdout=FAKE["github-pat"] + "\n"
            + json.dumps({"refresh_token": FAKE["gcp-refresh-token"]}),
            stderr="token=" + FAKE["google-oauth-access-token"])
        result = run_hook(event)
        self.assertEqual(result.returncode, 0, result.stderr)
        out = updated(result)
        self.assert_redacted(out["stdout"], "github-pat", "gcp-refresh-token")
        self.assert_redacted(out["stderr"], "google-oauth-access-token")
        self.assertIn("[REDACTED by gitleaks: google-oauth-access-token]", out["stderr"])
        self.assertEqual(out.keys(), event["tool_response"].keys())
        self.assertIs(out["interrupted"], False)

    def test_read_content_is_redacted_and_keeps_its_shape(self):
        content = ("[drive]\ntype = drive\n"
                   f"client_secret = {FAKE['gcp-client-secret']}\n"
                   f"pass = {FAKE['rclone-obscured-password']}\n")
        event = read_event(content)
        result = run_hook(event)
        self.assertEqual(result.returncode, 0, result.stderr)
        out = updated(result)
        self.assert_redacted(out["file"]["content"], "gcp-client-secret",
                             "rclone-obscured-password")
        self.assertIn("[REDACTED by gitleaks: rclone-obscured-password]", out["file"]["content"])
        self.assertIn("type = drive", out["file"]["content"])
        expected = {**event["tool_response"]["file"], "content": out["file"]["content"]}
        self.assertEqual(out, {"type": "text", "file": expected})

    def test_clean_output_is_left_alone(self):
        result = run_hook(bash_event("total 0\nREADME.md\n"))
        self.assertEqual((result.returncode, result.stdout), (0, ""), result.stderr)

    def test_binary_read_is_left_alone(self):
        event = {"tool_name": "Read", "tool_response": {
            "type": "image", "file": {"base64": FAKE["github-pat"], "type": "image/png"}}}
        result = run_hook(event, REDACT_SECRETS_GITLEAKS="/nonexistent/gitleaks")
        self.assertEqual((result.returncode, result.stdout), (0, ""), result.stderr)

    def assert_withheld(self, scanner):
        result = run_hook(read_event("pass = " + FAKE["rclone-obscured-password"]),
                          REDACT_SECRETS_GITLEAKS=scanner)
        self.assertEqual(result.returncode, 2)
        self.assertIn("withheld", result.stderr)
        out = updated(result)
        self.assertEqual(out["type"], "text")
        self.assertNotIn(FAKE["rclone-obscured-password"], json.dumps(out))
        self.assertEqual(out["file"]["content"],
                         "[output withheld: the gitleaks secret scan could not run]")

    def test_missing_scanner_withholds_the_output(self):
        self.assert_withheld("/nonexistent/gitleaks")

    def test_failing_scanner_withholds_the_output(self):
        self.assert_withheld("false")


if __name__ == "__main__":
    unittest.main()
