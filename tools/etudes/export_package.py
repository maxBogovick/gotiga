"""Make deploy/etude-set/ (etudes.json + load.py) from tools/etudes/etudes.py.

  python3 tools/etudes/export_package.py
"""
import json
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
import build  # noqa: E402
from etudes import ETUDES as _STUDIES  # noqa: E402
from meetings import MEETINGS  # noqa: E402

ETUDES = _STUDIES + MEETINGS

OUT = ROOT / "deploy" / "etude-set"


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    package = {"etudes": [{"slug": e["slug"], "sortOrder": e["sort_order"], "body": build.body_of(e)} for e in ETUDES]}
    (OUT / "etudes.json").write_text(json.dumps(package, ensure_ascii=False, indent=1))
    shutil.copy(HERE / "load.py", OUT / "load.py")
    print("etudes", len(ETUDES), "->", OUT)


if __name__ == "__main__":
    main()
