"""Do meetings reward the choice of cards? (guest teams: random / built against / built wrong)

  ADMIN_KEY=... python3 tools/etudes/check_meetings.py [games] [slug ...]
"""
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
from meetings import MEETINGS  # noqa: E402


DEFAULT_GAMES = 12     # per guest team; ~1 minute per meeting
HARD_CAP = 40          # more needs --force
SECONDS_PER_MEETING = 120


def main():
    args = [a for a in sys.argv[1:] if a != "--force"]
    force = "--force" in sys.argv
    games = args[0] if args else str(DEFAULT_GAMES)
    only = args[1:]
    if int(games) > HARD_CAP and not force:
        sys.exit(f"refused: {games} games per team is heavy (200 took 38 minutes). Up to {HARD_CAP} runs; more needs --force.")
    key = os.environ["ADMIN_KEY"]
    cards = subprocess.run(["curl", "-s", "-H", f"Authorization: Bearer {key}",
                            "http://localhost:3000/api/v1/admin/battles/cards"], capture_output=True, text=True).stdout
    Path("/tmp/etude_cards.json").write_text(cards)
    data = [{"slug": m["slug"], "botDepth": m["bot_depth"], "setup": m["setup"], "counter": m["counter"], "wrong": m["wrong"]}
            for m in MEETINGS if not only or m["slug"] in only]
    exe = str(ROOT / "src-tauri/server/target/release/examples/meeting_check")
    print(f"{'встреча':26} {'случайная':>10} {'против':>10} {'наоборот':>10}   ({games} игр на колоду, лимит {SECONDS_PER_MEETING} с на встречу)")
    for m in data:
        Path("/tmp/meetings.json").write_text(json.dumps([m]))
        try:
            out = subprocess.run([exe, "/tmp/etude_cards.json", "/tmp/meetings.json", games], capture_output=True, text=True,
                                 timeout=SECONDS_PER_MEETING).stdout
            print(out.splitlines()[-1] if out.strip() else f"{m['slug']}: нет вывода")
        except subprocess.TimeoutExpired:
            print(f"{m['slug']:26} прервано по времени ({SECONDS_PER_MEETING} с): уменьшите число игр")


if __name__ == "__main__":
    main()
