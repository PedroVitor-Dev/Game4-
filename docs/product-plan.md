# Plano do produto

## Visão

O Game4∞ deve ajudar pessoas e comunidades a manter acervos verificáveis, recuperáveis e utilizáveis, mesmo sem os serviços dos criadores do projeto. O foco inicial é Windows, com funcionamento local sem conta obrigatória.

## Públicos e necessidades

| Público | Necessidade |
|---|---|
| Colecionador | Catalogar arquivos e verificar alterações ou danos |
| Curador | Documentar origem, versões e condições de execução |
| Comunidade | Compartilhar coleções e manter cópias independentes |
| Guardião | Oferecer armazenamento e rede dentro de limites explícitos |

## Primeira entrega: biblioteca local

- Importar arquivos e pastas com progresso e cancelamento.
- Registrar tamanho, caminho, plataforma, versão e SHA-256.
- Detectar duplicatas exatas sem excluir arquivos automaticamente.
- Consultar e editar metadados locais em SQLite.
- Verificar novamente um arquivo e distinguir íntegro, alterado, ausente e não verificado.
- Exportar e importar um manifesto versionado, sem exigir internet.
- Orientar o usuário quando um arquivo mudar durante a leitura.

Aceitação: importar um acervo de teste, fechar e reabrir o app, verificar integridade, exportar o manifesto e reconstruir o catálogo em uma instalação limpa. Erros não devem interromper silenciosamente toda a operação.

## Alpha P2P

Adicionar torrents e magnet links, criação de torrents, fila, progresso, limites de velocidade, pausa, retomada, persistência e verificação das peças. Compartilhar exige ação explícita e explicação da exposição de rede.

Aceitação: duas instalações transferem um acervo autorizado, retomam após reinício e validam os arquivos finais. O aplicativo deve continuar útil quando não houver peers disponíveis.

## Diferenciais posteriores

1. Modo Guardião com seleção de coleções, quota de disco, limites de rede e horários.
2. Resgate de conteúdos com baixa disponibilidade observada.
3. Catálogos assinados e histórico de versões.
4. Documentação de execução e kit de reconstrução offline.
5. Redundância, recuperação e verificação periódica.
6. Provas temporais e financiamento opcional de acervos.

## Fora do escopo inicial

Moeda própria, NFTs, custódia financeira, recompensas automáticas por seed, marketplace, emulador embutido, execução automática, suporte simultâneo a múltiplos sistemas operacionais e formatos proprietários complexos.

## Indicadores de qualidade

Medir em acervos de teste: sucesso de restauração, detecção de corrupção, erros de importação, capacidade de retomada e consumo de memória. Registrar tamanho do acervo, hardware e versões para permitir comparação. Metas numéricas de desempenho serão definidas depois da primeira medição; não haverá telemetria pessoal por padrão.
