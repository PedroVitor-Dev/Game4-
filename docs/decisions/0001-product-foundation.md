# ADR 0001 — Fundação local e descentralização progressiva

Data: 03/11/2026. Estado: aceito como direção inicial.

## Contexto

O repositório começa com README e licença. A visão inclui preservação de arquivos, P2P e tecnologias descentralizadas. Implementar todas as integrações de início aumentaria o risco de não entregar uma ferramenta utilizável.

## Alternativas

1. Construir primeiro uma plataforma financeira com token e incentivos.
2. Depender de serviço central para catálogo, identidade e arquivos.
3. Entregar biblioteca local e evoluir a descentralização em módulos.

## Decisão

Adotar a alternativa 3. Seguir a stack prevista no README: Tauri 2, Rust, React, TypeScript, Tailwind CSS e SQLite. Usar BitTorrent inicialmente, escolhendo a engine após experimento. Manter formatos abertos, exportação offline e compartilhamento explícito.

IPFS, provas temporais, doações e armazenamento pago serão opcionais e posteriores. Não criar token próprio, custódia ou recompensas automáticas na fundação. O usuário controla o push; o trabalho será registrado em commits Conventional Commits.

## Consequências

A primeira versão será menor que a visão completa. Será possível validar importação, integridade e restauração antes de depender de redes externas. O núcleo precisa de limites claros para permitir adaptadores futuros. Detalhes de engine, formatos e chaves continuam em aberto.

## Evidência

Direção baseada no README e no alinhamento de planejamento com o responsável pelo projeto. Ainda não há protótipo, benchmark ou validação operacional da stack neste repositório.
