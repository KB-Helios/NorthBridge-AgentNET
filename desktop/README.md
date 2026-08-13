# NorthBridge AgentNET Desktop

This directory is the local-first Tauri v2 deployment target for QM. It establishes the native host, a verified bundled AgentGateway sidecar, staged Node 24 and QM resources, and a small local control surface. The wider desktop profile is described in [`../docs/desktop-local-first-architecture.md`](../docs/desktop-local-first-architecture.md).

## Current implementation boundary

| Capability | Status |
| --- | --- |
| Tauri v2 Windows host | Implemented and compiled locally. |
| AgentGateway v1.4.1 Windows sidecar | Implemented, SHA-256 verified, loopback smoke-tested, and started by the native host. |
| Local gateway control UI | Implemented with typed native commands for status, start, and stop. |
| Bundled Node 24 and QM resource staging | Implemented through `stage:qm`. |
| Existing QM Vite/Lit UI served by the bundled local runtime | Planned next integration. |
| SQLite stores and a SQLite-backed local queue | Planned persistence migration. |
| OS credential vault integration | Planned before provider credentials are enabled. |
| AgentOS Direct VM sandbox adapter | Deferred behind the existing QM `Sandbox` interface. |

## Local development

Run the following commands from this directory after installing Node 24, Rust, and the Windows desktop build prerequisites.

```powershell
npm install
npm run prepare:gateway
npm run stage:qm
npm run tauri dev
```

| Command | Purpose |
| --- | --- |
| `npm run prepare:gateway` | Downloads AgentGateway v1.4.1, verifies its published SHA-256 value, and prepares its target-specific sidecar filename. |
| `npm run stage:qm` | Copies the existing QM source, web UI, dependencies, and Node-version marker into desktop resources. |
| `npm run build` | Stages QM resources, type-checks the local control surface, and creates its web assets. |
| `npm run tauri build` | Produces the Windows desktop package after resources are staged. |

## Security model

The host writes AgentGateway configuration into per-user application data, starts it with a fixed executable and fixed `-f <config>` argument pattern, and uses loopback-only administrative and LLM ports. The web layer can request typed gateway lifecycle operations but never supplies an executable path or arbitrary shell arguments. Provider credentials are intentionally outside the current implementation; the production desktop profile must read them from operating-system-backed secret storage and inject them only into AgentGateway at launch.

## Validation performed

The local control surface passed TypeScript validation and Vite production build. The Rust host passed `cargo fmt --check`, `cargo check`, and a linked debug build. The verified AgentGateway binary served `/config_dump` on its loopback administration endpoint and was observed to start when the compiled Tauri executable launched.

## References

[Tauri external binaries](https://v2.tauri.app/develop/sidecar/)  
[AgentGateway standalone binary](https://agentgateway.dev/docs/standalone/latest/deployment/binary/)  
[AgentGateway diagnostics](https://agentgateway.dev/docs/standalone/latest/operations/debug/)
