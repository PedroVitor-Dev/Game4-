# Segurança planejada

Este documento descreve controles pretendidos, não uma auditoria ou declaração de segurança já alcançada.

| Ameaça | Controle requerido |
|---|---|
| Caminhos maliciosos e escape do destino | Validar caminhos relativos, caminhos Windows especiais e destino efetivo; bloquear travessia e redirecionamentos indevidos |
| Pacotes excessivos ou malformados | Limites de tamanho, quantidade, profundidade e descompressão; parsing defensivo |
| Conteúdo executável | Não executar automaticamente; ações de abertura explícitas |
| Manifestos adulterados | Validar esquema e hashes; assinaturas futuras com política de confiança |
| Exaustão de disco ou memória | Leitura em blocos, quotas, checagem de espaço e cancelamento |
| Corrupção ou interrupção | Estados persistentes, escrita temporária quando aplicável e recuperação após falha |
| Acesso excessivo pela interface | Comandos Tauri mínimos e validação no Rust; evitar HTML remoto privilegiado |
| Exposição involuntária de rede | Compartilhamento explícito e indicação de IP visível aos peers |
| Vazamento por logs e exportações | Remover segredos e evitar caminhos pessoais por padrão |
| Dependências comprometidas | Lockfiles, revisão de dependências e verificações no processo de release |

## Antes do alpha P2P

Verificar importação de nomes e caminhos maliciosos, cancelamento, arquivos mudando durante hash, falta de espaço, interrupção de escrita, manifests inválidos, reinício e transferência corrompida. Revisar portas, DHT, trackers e compartilhamento em segundo plano. Não prometer anonimato.

## Dados e direitos

Metadados podem conter informações pessoais; exportação e publicação devem ser conscientes. Licença do software não autoriza redistribuição dos acervos. Usar conteúdo sintético, próprio ou redistribuível nos testes e demonstrações.

## Relato de vulnerabilidades

Canal privado ainda não definido. Antes de distribuição pública, criar `SECURITY.md` com contato verificável e processo de resposta. Não inventar endereço de suporte nem solicitar publicação de segredos em issues.
