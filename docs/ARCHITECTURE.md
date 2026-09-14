# TOGARA OS Architecture

```text
                    TOGARA SHELL
                         │
                 REALM RUNTIME LAYER
                         │
             ┌───────────┼───────────┐
             │           │           │
          IDENTITY    EVENT FABRIC  AGENTS
             │           │           │
             └───────────┼───────────┘
                         │
                    OS SERVICES
                         │
          ┌──────────────┼──────────────┐
          │              │              │
      FILESYSTEM      NETWORK       SECURITY
          │              │              │
          └──────────────┼──────────────┘
                         │
                    TOGARA KERNEL
                         │
      ┌──────────────────┼──────────────────┐
      │                  │                  │
   SCHEDULER          MEMORY               IPC
      │                  │                  │
      └──────────────────┼──────────────────┘
                         │
                   HARDWARE / UEFI
```

The current MVP implements the bootloader → kernel → framebuffer portion of this stack. The upper layers are intentionally staged for subsequent development.
