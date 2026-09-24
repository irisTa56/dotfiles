---
description: A finding that asks for a one-line docstring, cheap to add, with nothing going wrong without it, raised by the tech lead as must-fix
max_turns: 8
allowed_tools: [Skill]
---

Judge this review finding on my change by running the address-finding skill. You cannot edit files here, so reply with your verdict and, if you would change anything, what you would change.

The purpose of my change is to stop sign-up from creating a second account for the same address typed with different capitalization or spacing.

The change adds this private helper to `accounts.py` and calls it in `sign_up` and `find_account` before any lookup:

```python
def _normalize_email(address: str) -> str:
    return address.strip().lower()
```

The other private helpers in `accounts.py` have no docstrings.

The finding comes from the team's tech lead, who marked it "must fix before merge": "Add a docstring to `_normalize_email`. It is a one-line change and makes the helper self-documenting."
