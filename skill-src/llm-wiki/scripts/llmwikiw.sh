#!/bin/sh
set -eu

if [ -n "${LLMWIKI_BIN:-}" ]; then
  exec "$LLMWIKI_BIN" "$@"
fi

if [ -n "${LLMWIKI_INSTALL_PATH:-}" ] && [ -x "$LLMWIKI_INSTALL_PATH" ]; then
  exec "$LLMWIKI_INSTALL_PATH" "$@"
fi

if [ -n "${XDG_DATA_HOME:-}" ]; then
  shared="$XDG_DATA_HOME/llmwiki/bin/llmwiki"
else
  shared="$HOME/.local/share/llmwiki/bin/llmwiki"
fi

if [ -x "$shared" ]; then
  exec "$shared" "$@"
fi

if command -v llmwiki >/dev/null 2>&1; then
  exec llmwiki "$@"
fi

echo "llmwiki is not installed in the shared location. Run \`llmwiki install\` first." >&2
exit 1
