# Bobola 1.1.1 — Music companion reliability

Bobola 1.1.1 fixes the Music / YuTuTui companion startup path and improves the Music panel's loading, error, retry, and wide-window layout states.

The Music panel now starts the bundled YuTuTui daemon in the background when needed, waits for its local control endpoint, and exposes a retry action when the controller cannot be reached. Windows upgrades also stop an existing YuTuTui process before replacing its bundled executable.
