# Cronograma e marcos

Base: 03/11/2026. Estimativa inicial de 16 semanas ativas para um alpha P2P, seguida de 4 semanas de reserva. Pressupõe um desenvolvedor principal com dedicação regular e revisão do responsável pelo projeto. Não foi validada por velocidade histórica da equipe.

As semanas contam a partir do início efetivo da implementação. Feriados, disponibilidade, incompatibilidades e mudanças de escopo podem deslocar as datas. Revisar a previsão ao concluir cada marco; não substituir critérios de qualidade por uma data.

| Marco | Semanas | Entrega | Critério de saída |
|---|---|---|---|
| M0 — Fundação | 1–2 | Tauri, React, TypeScript, Rust, CI e interface base | Build e execução em Windows limpo; verificação automatizada básica |
| M1 — Biblioteca | 3–5 | Importação, SQLite, hashes e consulta | Persistência após reinício; cancelamento e arquivos problemáticos tratados |
| M2 — Preservação local | 6–7 | Manifesto v0, exportação e verificação | Reconstrução do catálogo e detecção de corrupção demonstradas |
| M3 — Prova P2P | 8–9 | Experimento de engine BitTorrent | Transferência entre duas instalações; licença e integração avaliadas |
| M4 — Transferências | 10–12 | Fila, torrents, magnets, limites e retomada | Reinício sem perda do estado e integridade final validada |
| M5 — Alpha Windows | 13–16 | Integração, testes, instalador e documentação | Instalação limpa, restauração e revisão de segurança concluídas |
| Reserva | 17–20 | Correções e compatibilidade | Usar conforme riscos observados; não preencher com novos recursos |

## Dependências e alternativas

- M0 depende de ferramentas e SDKs Windows disponíveis.
- M1 define o modelo utilizado por M2; M3 pode exigir mudanças na integração, mas não deve acoplar o catálogo à engine.
- M3 é um ponto de decisão: se nenhuma engine atender aos critérios, publicar primeiro uma versão de biblioteca local e replanejar o P2P.
- `.g4` e `.g4t` exigem especificação e fixtures antes da implementação. Não são bloqueadores da prova P2P; usar `.torrent` e manifestos JSON enquanto isso.
- Alpha não significa estabilidade de formato. Migrações e limitações devem ser documentadas.

## Horizonte posterior, sem data comprometida

| Fase | Escopo | Entrada necessária |
|---|---|---|
| Comunidades | Catálogos assinados, histórico e espelhamento | Alpha utilizável e formato de manifesto validado |
| Guardiões | Quotas, agendamento, saúde e resgate | Observação de disponibilidade e controle de compartilhamento confiáveis |
| Longo prazo | Recuperação, kit offline, IPFS e provas temporais | Testes de restauração e política de compatibilidade |
| Financiamento | Doações e patrocínio; estudo de armazenamento pago | Necessidade demonstrada, custos e responsabilidades definidos |

## Cadência

Planejar um incremento pequeno por vez. Ao concluir: validar, atualizar o diário, registrar riscos novos e fazer commit. A cada marco: comparar estimativa com esforço observado, revisar backlog e apresentar a próxima entrega. Publicação e push permanecem sob controle do responsável pelo projeto.
