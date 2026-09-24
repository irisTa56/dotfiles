import sys

from settings import load_settings, save_settings


def set_option(key: str, value: str) -> int:
    settings = load_settings()
    settings[key] = value
    if save_settings(settings):
        print(f"Saved {key}={value}")
        return 0
    print(f"Could not save {key}", file=sys.stderr)
    return 1
