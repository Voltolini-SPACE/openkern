#!/bin/bash
# Constrói "OpenKern — Panel.app" na Mesa, padrão DESKTOP_LAUNCHER_STANDARD V1:
# healthcheck -> abrir; senão iniciar idempotente -> esperar -> abrir.
# Bind 127.0.0.1. Sem segredos. Log em ~/Library/Logs/PantheonLaunchers/.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/../.." && pwd)"
APP="$HOME/Desktop/OpenKern — Panel.app"
PORT=8150

# icns a partir da geometria congelada (build-time: python3 do PATH, que tem PIL;
# o runtime do servidor usa /usr/bin/python3 e é stdlib-only)
python3 "$REPO/brand/exports/gen_icns.py" /tmp/openkern_appicon.icns

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp /tmp/openkern_appicon.icns "$APP/Contents/Resources/appicon.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>OpenKern — Panel</string>
  <key>CFBundleDisplayName</key><string>OpenKern — Panel</string>
  <key>CFBundleIdentifier</key><string>space.voltolini.openkern.panel</string>
  <key>CFBundleVersion</key><string>1.0</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleExecutable</key><string>launcher</string>
  <key>CFBundleIconFile</key><string>appicon</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
</dict></plist>
PLIST

cat > "$APP/Contents/MacOS/launcher" <<LAUNCH
#!/bin/bash
# OpenKern — Panel · launcher V1 (healthcheck -> abrir; senão iniciar -> esperar -> abrir)
export PATH="/usr/bin:/bin:/usr/sbin:/sbin"
REPO="$REPO"
PORT=$PORT
URL="http://127.0.0.1:\$PORT/"
LOGDIR="\$HOME/Library/Logs/PantheonLaunchers"
LOG="\$LOGDIR/openkern-panel.log"
mkdir -p "\$LOGDIR"
ts(){ date "+%Y-%m-%d %H:%M:%S"; }

if /usr/bin/curl -s --max-time 2 "http://127.0.0.1:\$PORT/api/health" >/dev/null 2>&1; then
  echo "\$(ts) healthcheck OK -> open" >> "\$LOG"
  /usr/bin/open "\$URL"; exit 0
fi

echo "\$(ts) iniciando servidor" >> "\$LOG"
nohup /usr/bin/python3 "\$REPO/panel/server.py" --port "\$PORT" >> "\$LOG" 2>&1 &

for i in \$(seq 1 30); do
  if /usr/bin/curl -s --max-time 1 "http://127.0.0.1:\$PORT/api/health" >/dev/null 2>&1; then
    echo "\$(ts) pronto na tentativa \$i -> open" >> "\$LOG"
    /usr/bin/open "\$URL"; exit 0
  fi
  sleep 0.5
done

echo "\$(ts) BLOCKED: servidor nao respondeu em 15s" >> "\$LOG"
/usr/bin/osascript -e 'display notification "Servidor não respondeu em 15s. Ver ~/Library/Logs/PantheonLaunchers/openkern-panel.log" with title "OpenKern — Panel"'
exit 70
LAUNCH

chmod +x "$APP/Contents/MacOS/launcher"
touch "$APP"
echo "launcher criado: $APP"
