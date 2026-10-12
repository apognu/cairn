# cli

The yourdocs binary. Only `dev` exists so far; serve, migrate, export, backup, restore and admin come later.
Not obvious: it holds no logic, it parses arguments and calls `yourdocs-server`.
