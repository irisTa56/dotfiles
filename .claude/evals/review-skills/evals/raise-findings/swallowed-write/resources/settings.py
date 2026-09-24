import json
from pathlib import Path

SETTINGS_PATH = Path.home() / ".todo" / "settings.json"


def load_settings(path: Path = SETTINGS_PATH) -> dict:
    if not path.exists():
        return {}
    return json.loads(path.read_text())


def save_settings(settings: dict, path: Path = SETTINGS_PATH) -> bool:
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(settings, indent=2))
    except OSError:
        pass
    return True
