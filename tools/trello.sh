#!/usr/bin/env bash
# trello.sh — the WriftHeart bug-board helper (To Do -> Doing -> Done).
#
# Creds live in the macOS KEYCHAIN, not in any repo/.env — retrieve with the
# `-a runfiller` account flag (this is the piece that trips other threads up).
# In the sandbox: force IPv4 (`curl -4`, no IPv6) and NEVER send a GET body
# (Trello's edge 403s an HTML page) — use `-G --data-urlencode`.
#
# Usage:
#   tools/trello.sh bugs                 # list open cards (To Do + Doing)
#   tools/trello.sh start <shortLink>    # move a card to Doing
#   tools/trello.sh done  <shortLink> "what I changed"   # move to Done + comment
set -euo pipefail
KEY="$(security find-generic-password -s trello-api-key -a runfiller -w)"
TOKEN="$(security find-generic-password -s trello-token -a runfiller -w)"
BOARD=6a5cfde59d2c53e701896380
TODO=6a5cfde59d2c53e701896404
DOING=6a5cfde59d2c53e701896405
DONE=6a5cfde59d2c53e701896406

get()  { curl -4 -s -G "https://api.trello.com/1$1" --data-urlencode "key=$KEY" --data-urlencode "token=$TOKEN" "${@:2}"; }
put()  { curl -4 -s -X PUT  "https://api.trello.com/1$1" --data-urlencode "key=$KEY" --data-urlencode "token=$TOKEN" "${@:2}" >/dev/null; }
post() { curl -4 -s -X POST "https://api.trello.com/1$1" --data-urlencode "key=$KEY" --data-urlencode "token=$TOKEN" "${@:2}" >/dev/null; }

case "${1:-bugs}" in
  bugs)
    for L in "$TODO:TO DO" "$DOING:DOING"; do
      lid=${L%%:*}; lname=${L#*:}
      echo "== $lname =="
      get "/lists/$lid/cards" --data-urlencode "fields=name,desc,shortLink" | python3 -c '
import sys,json
cs=json.load(sys.stdin)
if not cs: print("  (empty)")
for c in cs:
    d=(c.get("desc") or "").replace("\n"," ").strip()
    print("  ["+c["shortLink"]+"] "+c["name"]+((" :: "+d[:120]) if d else ""))'
    done ;;
  start) put "/cards/$2" --data-urlencode "idList=$DOING"; echo "-> Doing: $2" ;;
  done)
    put "/cards/$2" --data-urlencode "idList=$DONE"
    [ -n "${3:-}" ] && post "/cards/$2/actions/comments" --data-urlencode "text=$3"
    echo "-> Done: $2" ;;
  *) echo "usage: trello.sh {bugs|start <card>|done <card> [comment]}"; exit 1 ;;
esac
