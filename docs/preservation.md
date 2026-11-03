# Preservação e formatos

## Princípios

Preservar os bytes originais e registrar transformações em cópias separadas. Um hash detecta diferenças; sozinho não comprova autenticidade histórica, autoria ou autorização de distribuição. Disponibilidade observada não é prova de existência de cópias independentes.

## Manifesto inicial

Projetar primeiro um manifesto JSON versionado, com esquema público e exemplos. Campos candidatos: versão do esquema, identificador da coleção, caminhos relativos, tamanho, SHA-256, plataforma, edição, idioma, origem declarada, licença declarada e instruções de execução.

Metadados declarados devem ser identificados como tal. Separar campos obrigatórios de extensões. Rejeitar versões incompatíveis explicitamente. Definir normalização de caminhos, Unicode e serialização canônica antes de assinar documentos. Nunca exportar caminhos absolutos pessoais por padrão.

## Formatos propostos

| Formato | Papel | Situação |
|---|---|---|
| `.g4` | Pacote portátil com manifesto e arquivos | Conceitual; contêiner e limites a definir |
| `.g4t` | Referências de compartilhamento e metadados | Conceitual; justificar vantagem sobre `.torrent` + JSON |
| `.g4lib` | Exportação de catálogo e referências | Futuro; não implica inclusão dos jogos |

Não fixar extensões como formatos estáveis antes de especificar segurança, compatibilidade e migrações. Evitar recompressão obrigatória que dificulte acesso aleatório, retomada e interoperabilidade com torrents.

## Integridade e recuperação

Verificar ao importar, após transferência e sob demanda. Agendamento futuro deve respeitar disco e energia. Exibir data da última verificação e distinguir arquivo inacessível de arquivo corrompido. Reparação usa cópias verificadas ou redundância disponível; nunca sobrescrever o único original silenciosamente.

## Preservação utilizável

Registrar manuais autorizados, dependências, versões de ferramentas, configurações e instruções de execução. Incluir arquivos auxiliares apenas quando redistribuíveis. Não executar conteúdo automaticamente.

## Teste de sobrevivência

Exportar uma coleção de teste com manifesto, esquema, instruções e hashes. Em ambiente separado e offline, reconstruir o catálogo e verificar os arquivos sem consultar servidores do Game4∞. Repetir o teste após mudanças relevantes de formato. Assinaturas e provas temporais devem viajar junto com a exportação quando existirem.
