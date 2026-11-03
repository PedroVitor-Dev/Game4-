# Descentralização e financiamento

## Direção

As funções essenciais não exigem conta, carteira, token ou pagamento. BitTorrent é o transporte inicial. Identidade de curadores, integridade dos arquivos, disponibilidade e financiamento são problemas distintos.

## Modo Guardião e resgate

Planejar adesão explícita por coleção, quota de disco, limites de upload/download, horários e saída voluntária. Não apagar arquivos pessoais para liberar quota. Informar o que permanece compartilhado ao pausar ou remover uma coleção.

O painel deve mostrar fontes observadas, última observação e verificações recentes. Classificar risco com base nesses sinais, sem declarar contagem global de cópias. Priorizar resgate de conteúdos com poucas fontes e evitar que todos os guardiões escolham apenas os mesmos itens populares. Resistência a identidades falsas e relatos falsos precisa de pesquisa antes de qualquer incentivo econômico.

## Catálogos assinados

Assinaturas permitem verificar integridade e vínculo com uma chave. Planejar exportação, rotação, recuperação e revogação de chaves. Confiança é local: usuários escolhem curadores e comunidades; não haverá autoridade editorial global obrigatória.

## Integrações

| Tecnologia | Uso proposto | Condição de adoção |
|---|---|---|
| IPFS | Distribuição complementar de manifestos e conteúdos | Testar pinning, restauração e consumo de recursos |
| OpenTimestamps / Bitcoin | Prova temporal de manifestos | Preservar e verificar a prova junto com o manifesto |
| Bitcoin / Lightning | Doação voluntária para projeto ou acervo | Preferir fluxo externo; sem custódia de chaves pelo app |
| Filecoin | Cópia adicional financiada | Experimento com contrato, prazo, custo e recuperação real |

IPFS requer retenção efetiva por nós; identificação por conteúdo não garante persistência. OpenTimestamps não armazena os jogos nem certifica autoria. Filecoin exige planejamento de armazenamento e recuperação; não prometer permanência.

## Patrocínio de acervos

Hipótese futura: apresentar objetivo, orçamento, período, responsáveis, política de encerramento e relatórios de verificação. Financiamento recebido não equivale automaticamente a armazenamento comprovado. Começar com processo transparente e contribuição externa, antes de considerar contratos inteligentes.

## Exclusões iniciais

Não criar token próprio, NFTs, mineração no dispositivo, custódia, rendimento financeiro ou pagamento automático por tráfego. Esses mecanismos ampliariam a superfície de fraude e desviariam o foco da restauração verificável.

## Referências técnicas

- [Persistência no IPFS](https://docs.ipfs.tech/concepts/persistence/)
- [OpenTimestamps](https://opentimestamps.org/)
- [Especificação Filecoin](https://spec.filecoin.io/)
- [BTCPay Server](https://docs.btcpayserver.org/FAQ/General/)

Referências consultadas no planejamento de 03/11/2026. Revalidar APIs, custos, licenças e condições quando uma integração entrar em implementação.
