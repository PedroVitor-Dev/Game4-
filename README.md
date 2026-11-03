
<div align="center">

# 🎮 Game4∞

### Preserve the Past. Share the Future.

**Uma rede aberta e descentralizada para a preservação digital dos videogames.**

![Status](https://img.shields.io/badge/status-em%20desenvolvimento-yellow)
![License](https://img.shields.io/badge/license-GPLv3-blue)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey)
![Open Source](https://img.shields.io/badge/open%20source-❤️-red)

---

**Preservar. Compartilhar. Descentralizar.**

</div>

## 🌌 Sobre o projeto

O **Game4∞** é um projeto open source dedicado à preservação digital de videogames por meio de tecnologias descentralizadas e redes peer-to-peer (P2P).

Nossa missão é desenvolver ferramentas que permitam às comunidades preservar, organizar, verificar e compartilhar arquivos digitais de maneira independente, sem depender exclusivamente de servidores centralizados ou plataformas comerciais.

Acreditamos que a história dos videogames merece ser preservada para as próximas gerações.

O Game4∞ não pretende ser apenas um aplicativo de downloads, mas uma infraestrutura aberta para preservação digital de longo prazo.

> **Nossa visão:** construir uma tecnologia que continue funcionando mesmo que seus criadores deixem de mantê-la.

---

## 🎯 Objetivos

- 🌍 Incentivar a preservação histórica dos videogames.
- 🔗 Desenvolver uma infraestrutura descentralizada.
- 📦 Criar formatos abertos para preservação digital.
- 🛡️ Garantir a integridade dos arquivos por meio de hashes criptográficos.
- 🌱 Permitir que comunidades mantenham seus próprios acervos.
- 💻 Disponibilizar ferramentas gratuitas e open source.
- ♾️ Reduzir a dependência de servidores e organizações centralizadas.
- 🤝 Promover colaboração e interoperabilidade.

---

## 🚧 Status do desenvolvimento

**Fase atual: Planejamento e desenvolvimento inicial — v0.1.0-alpha**

O Game4∞ ainda está em desenvolvimento.

As funcionalidades descritas neste documento representam os objetivos técnicos e a arquitetura planejada. Elas não devem ser interpretadas como recursos já implementados.

A primeira versão terá como foco um aplicativo desktop para Windows, capaz de gerenciar arquivos locais e realizar transferências P2P.

---

## ⚡ Funcionalidades planejadas

### 📚 Biblioteca local

- Importação de arquivos e diretórios.
- Organização por plataforma e categoria.
- Identificação de tamanho e formato.
- Geração de hashes SHA-256.
- Armazenamento local de metadados.
- Verificação de integridade.
- Exportação de manifestos.

### 🌐 Compartilhamento P2P

O Game4∞ utilizará tecnologias compatíveis com o ecossistema BitTorrent para permitir transferências descentralizadas.

Funcionalidades previstas:

- Criação e importação de torrents.
- Suporte a magnet links.
- Descoberta de peers via DHT.
- Compatibilidade com trackers.
- Download e upload simultâneos.
- Gerenciamento de seeders e leechers.
- Pausa e retomada de transferências.
- Monitoramento de velocidade.
- Limites de upload e download.
- Verificação de integridade das peças.

Os arquivos serão transferidos diretamente entre os participantes sempre que a conectividade permitir, sem depender de servidores do Game4∞ para transportar seu conteúdo.

### 🔐 Privacidade e controle

- Funcionamento sem cadastro obrigatório.
- Ausência de telemetria pessoal por padrão.
- Compartilhamento iniciado mediante ação explícita.
- Controle dos arquivos disponibilizados.
- Configurações locais.
- Transparência sobre conexões P2P.

**Nota:** conexões BitTorrent podem revelar o endereço IP público dos participantes aos demais peers.

---

## 📦 Formatos Game4∞

O projeto pretende desenvolver formatos próprios, abertos e documentados para facilitar a preservação e o compartilhamento de arquivos.

### `.g4` — Game4 Package

Formato de pacote portátil de preservação digital.

Poderá conter:

- Arquivos originais.
- Manifesto de preservação.
- Informações históricas.
- Documentação.
- Imagens e capas autorizadas.
- Hashes de integridade.

Estrutura conceitual:

```text
example.g4
├── manifest.json
├── game/
│   └── original-file.bin
├── artwork/
├── docs/
└── checksums.sha256
```

O formato deverá preservar a integridade dos arquivos originais e possuir uma especificação pública.

### `.g4t` — Game4 Torrent

Formato de manifesto para descoberta e compartilhamento P2P.

Poderá conter:

- Identificador do conteúdo.
- Hashes de integridade.
- Metadados básicos.
- Referências BitTorrent.
- Magnet links.
- Informações de versão.

O arquivo `.g4t` não precisará conter o jogo propriamente dito.

### `.g4lib` — Game4 Library

Formato planejado para as próximas fases.

Permitirá:

- Exportação de bibliotecas.
- Importação de catálogos.
- Espelhamento comunitário.
- Sincronização de metadados.
- Verificação de assinaturas digitais.

### Princípios dos formatos

Todos os formatos Game4∞ deverão priorizar:

1. Especificações públicas.
2. Compatibilidade entre implementações.
3. Integridade verificável.
4. Preservação dos arquivos originais.
5. Independência de serviços proprietários.
6. Compatibilidade futura.

---

## 🏗️ Arquitetura planejada

O Game4∞ utilizará uma arquitetura modular, permitindo a evolução independente dos componentes.

| Componente | Tecnologia |
|---|---|
| Desktop | Tauri 2 |
| Backend local | Rust |
| Frontend | React |
| Linguagem da interface | TypeScript |
| Estilização | Tailwind CSS |
| Banco de dados | SQLite |
| Transferência P2P | BitTorrent |
| Descoberta de peers | DHT / Trackers |
| Integridade | SHA-256 |
| Metadados | JSON |
| Distribuição | GitHub Releases |

A implementação da engine BitTorrent será definida após testes de compatibilidade, desempenho, segurança e licenciamento.

### Estrutura prevista

```text
Game4Infinity/
├── apps/
│   └── desktop/
│       ├── src/
│       │   ├── components/
│       │   ├── pages/
│       │   ├── hooks/
│       │   └── services/
│       └── src-tauri/
│           └── src/
│               ├── commands/
│               ├── database/
│               ├── library/
│               ├── torrent/
│               └── security/
├── packages/
│   └── manifest-schema/
├── docs/
│   ├── architecture.md
│   ├── protocol.md
│   └── security.md
├── tests/
├── README.md
├── CONTRIBUTING.md
└── LICENSE
```

---

## 🗺️ Roadmap

### Fase 1 — Fundação P2P

**Objetivo:** desenvolver um cliente desktop funcional para preservação e compartilhamento descentralizado.

- [ ] Configuração do projeto Tauri + React + Rust.
- [ ] Interface inicial do Game4∞.
- [ ] Biblioteca local.
- [ ] Banco de dados SQLite.
- [ ] Geração de hashes SHA-256.
- [ ] Criação de manifestos.
- [ ] Implementação inicial do formato `.g4`.
- [ ] Implementação inicial do formato `.g4t`.
- [ ] Integração da engine BitTorrent.
- [ ] Criação e importação de torrents.
- [ ] Suporte a magnet links.
- [ ] Descoberta de peers via DHT.
- [ ] Upload e download P2P.
- [ ] Gerenciamento de seeds e leechers.
- [ ] Persistência de transferências.
- [ ] Verificação de integridade.
- [ ] Testes de segurança.
- [ ] Publicação da versão alpha para Windows.

### Fase 2 — Comunidades descentralizadas

**Objetivo:** permitir a criação de bibliotecas independentes.

- [ ] Implementação do formato `.g4lib`.
- [ ] Catálogos comunitários.
- [ ] Importação e exportação de bibliotecas.
- [ ] Assinaturas digitais.
- [ ] Descoberta de catálogos.
- [ ] Sincronização de metadados.
- [ ] Ferramentas de moderação.
- [ ] Espelhamento de bibliotecas.
- [ ] Interoperabilidade entre comunidades.

### Fase 3 — Preservação de longo prazo

**Objetivo:** aumentar a resiliência e a disponibilidade dos acervos.

- [ ] Integração opcional com IPFS.
- [ ] Nós guardiões de preservação.
- [ ] Replicação voluntária.
- [ ] Indicadores de disponibilidade.
- [ ] Identificação de arquivos com poucas cópias.
- [ ] Verificações periódicas de integridade.
- [ ] Mecanismos de recuperação e redundância.
- [ ] Integração opcional com OpenTimestamps.
- [ ] Ancoragem criptográfica no Bitcoin.
- [ ] Pesquisa sobre incentivos comunitários de armazenamento.

---

## ♾️ Preservação descentralizada

A preservação digital não deve depender exclusivamente da existência de um único site, servidor ou organização.

O Game4∞ buscará permitir que:

- Arquivos sejam replicados entre participantes.
- Catálogos sejam exportados e reconstruídos.
- Metadados possam ser verificados independentemente.
- Comunidades mantenham infraestrutura própria.
- O software continue funcionando sem seus desenvolvedores originais.

**Descentralização não significa armazenamento eterno garantido.**

A disponibilidade de arquivos dependerá da existência de cópias acessíveis, da integridade dos dados e da participação contínua da comunidade.

---

## 🔗 Bitcoin e tecnologias descentralizadas

O Game4∞ poderá utilizar tecnologias adicionais para fortalecer a verificabilidade dos acervos.

### Bitcoin

A blockchain Bitcoin poderá ser utilizada indiretamente para registrar provas criptográficas de existência de determinados dados.

### OpenTimestamps

Integração futura para produzir provas de existência verificáveis sem armazenar os arquivos completos na blockchain.

### IPFS

Tecnologia considerada para identificação de conteúdo e replicação complementar.

Essas integrações serão opcionais.

O Game4∞ não exigirá criptomoedas, mineração ou pagamentos para utilizar suas funcionalidades essenciais.

---

## ⚖️ Direitos autorais

O Game4∞ é uma ferramenta tecnológica de propósito geral, voltada à preservação digital e ao compartilhamento P2P.

O projeto não concede direitos de reprodução ou distribuição sobre obras protegidas por direitos autorais.

Os usuários são responsáveis por verificar se possuem autorização ou outra base legal aplicável para compartilhar os conteúdos.

O projeto incentiva a preservação e a distribuição de:

- Jogos open source.
- Homebrews.
- Obras em domínio público.
- Jogos disponibilizados pelos próprios titulares.
- Conteúdos com licenças que permitam redistribuição.
- Documentação e materiais históricos distribuídos legalmente.

O fato de um jogo ser antigo, raro ou não estar mais disponível comercialmente não significa, por si só, que sua distribuição seja autorizada.

---

## 🤝 Contribuições

O Game4∞ pretende ser desenvolvido de forma aberta e colaborativa.

Contribuições poderão incluir:

- Desenvolvimento de funcionalidades.
- Correção de bugs.
- Melhorias de desempenho.
- Testes de segurança.
- Documentação.
- Traduções.
- Design e experiência do usuário.
- Pesquisa sobre preservação digital.

As diretrizes de contribuição serão documentadas em `CONTRIBUTING.md`.

---

## 🔒 Segurança

A segurança é uma prioridade do projeto.

Diretrizes previstas:

- Não executar automaticamente arquivos recebidos.
- Validar caminhos de arquivos e diretórios.
- Prevenir escrita fora dos diretórios autorizados.
- Verificar a integridade dos downloads.
- Solicitar confirmação antes de compartilhar arquivos.
- Evitar coleta desnecessária de informações.
- Manter dependências atualizadas.
- Documentar vulnerabilidades e correções.

---

## 📜 Licença

O **Game4∞** é distribuído sob a licença **GNU General Public License versão 3 ou posterior (GPL-3.0-or-later)**.

Isso permite:

- Utilizar o software para qualquer finalidade.
- Estudar seu funcionamento.
- Modificar seu código-fonte.
- Redistribuir cópias.
- Distribuir versões modificadas.
- Utilizar o software comercialmente.

A redistribuição do programa e de versões derivadas deverá respeitar as condições da GPL, incluindo as obrigações aplicáveis de disponibilização do código-fonte correspondente.

Consulte o arquivo [LICENSE](LICENSE) para o texto integral da licença.

A licença do software não se estende automaticamente aos jogos ou arquivos compartilhados por seus usuários.

---

## 💜 Filosofia

Videogames são mais do que entretenimento.

São parte da história da tecnologia, da arte, da música e da cultura digital.

Preservar essa história exige ferramentas abertas, interoperáveis e capazes de sobreviver às mudanças tecnológicas.

O Game4∞ nasce da ideia de que a preservação digital deve ser uma responsabilidade compartilhada.

**Nenhum servidor deveria ser o único guardião da história.**

---

<div align="center">

### 🎮 Game4∞

**Preserve the Past. Share the Future.**

*Construído para preservar. Projetado para durar.*

**Open Source • P2P • Digital Preservation**

</div>
