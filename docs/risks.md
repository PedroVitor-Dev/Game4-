# Registro de riscos

Avaliação inicial qualitativa em 03/11/2026. Responsável pela revisão técnica: manutenção do projeto; decisões de orçamento e publicação: responsável pelo projeto.

| ID | Risco | Impacto | Mitigação / gatilho |
|---|---|---|---|
| R01 | Escopo maior que capacidade | Alto | Entregar biblioteca antes das integrações; revisar a cada marco |
| R02 | Engine P2P incompatível ou inadequada | Alto | Prova real e revisão de licença em M3; alternativa: release local |
| R03 | Formatos próprios impedirem interoperabilidade | Alto | JSON e `.torrent` primeiro; especificação e fixtures antes de estabilizar |
| R04 | Poucos peers e desaparecimento de acervos | Alto | Mostrar indisponibilidade; guardiões e teste de restauração posteriores |
| R05 | Medidas de disponibilidade manipuladas | Alto | Exibir observações com data; não remunerar métricas não confiáveis |
| R06 | Falhas de disco, caminhos ou parsing | Alto | Revisão de segurança, limites e cenários de interrupção |
| R07 | Custos ou dependência de redes pagas | Alto | Integração opcional; orçamento e restauração testados |
| R08 | Perda de chaves de curadores | Alto | Projetar rotação e recuperação antes de assinaturas em produção |
| R09 | Ambiente Windows e instalador frágeis | Médio | Build e instalação em ambiente limpo em M0 e M5 |
| R10 | Metadados incorretos ou redistribuição indevida | Alto | Registrar procedência declarada e separar confiança de integridade |
| R11 | Estimativa sem histórico de execução | Alto | Reestimar após M0/M1; usar semanas ativas e reserva explícita |

Registrar ocorrência, responsável específico e ação quando um risco se materializar. Alterações de cronograma e arquitetura devem apontar o risco que motivou a revisão.
