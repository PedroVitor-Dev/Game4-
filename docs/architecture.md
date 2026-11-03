# Arquitetura planejada

Estado: direção aprovada para iniciar; detalhes serão ajustados por experimentos.

| Camada | Tecnologia / responsabilidade |
|---|---|
| Interface | React, TypeScript e Tailwind CSS; biblioteca, tarefas e transferências |
| Aplicativo | Tauri 2; comandos explicitamente expostos à interface |
| Núcleo local | Rust; importação, hashes, formatos, políticas e tarefas |
| Persistência | SQLite com migrações versionadas |
| Transferência | Adaptador BitTorrent; engine ainda não escolhida |
| Integrações futuras | Adaptadores independentes para IPFS e provas temporais |

## Limites

A interface solicita operações; o núcleo valida entradas e aplica permissões. A engine de transferência não deve controlar o modelo do catálogo. Falhas de rede não devem impedir a consulta da biblioteca local. Operações longas executam fora da thread da interface, com eventos de progresso e cancelamento cooperativo.

## Fluxo de importação

Selecionar caminho → enumerar com limites → registrar tarefa → ler arquivo em blocos → calcular hash → verificar estabilidade do arquivo → persistir resultado → apresentar resumo e falhas individuais.

Não carregar jogos inteiros em memória. Links simbólicos, junctions e ciclos precisam de política explícita; inicialmente não seguir diretórios redirecionados automaticamente. Reinícios devem permitir repetir operações sem duplicar registros inadvertidamente.

## Entidades iniciais

- Arquivo: identidade interna, tamanho, hash e estado de verificação.
- Localização: caminho e vínculo com arquivo; um conteúdo pode ter várias localizações.
- Item de acervo: título, plataforma, versão e referências aos arquivos.
- Manifesto: versão do esquema, entradas, metadados e identificadores.
- Tarefa: tipo, estado, progresso e erro recuperável.
- Transferência: identificador BitTorrent, destino e estado; separado do acervo.

Detalhar o esquema em uma migração antes de implementá-lo. Hash do conteúdo e identidade editorial são conceitos diferentes: duas versões do mesmo jogo podem ter hashes distintos.

## Estrutura prevista

```text
apps/desktop/src/           Interface
apps/desktop/src-tauri/src/ Núcleo Rust e adaptadores
packages/manifest-schema/  Esquemas e exemplos interoperáveis
docs/decisions/            Decisões de arquitetura
tests/fixtures/            Amostras sintéticas ou redistribuíveis
```

## Escolha da engine

Criar ADR comparando licença compatível com a distribuição, integração Rust/Tauri, manutenção, DHT, magnets, trackers, retomada, limites de recursos e comportamento em Windows. Não escolher exclusivamente por popularidade. Executar uma transferência e uma retomada reais antes da decisão.

## Verificação e distribuição

Verificar regras de domínio e parsing com testes; testar integração de SQLite, arquivos e transferências. Validar interface e instalador em Windows. Registrar versões das ferramentas. Nunca incluir jogos comerciais, chaves ou dados pessoais nas fixtures. Atualização automática e assinatura do instalador exigem decisão posterior sobre infraestrutura e chaves.
