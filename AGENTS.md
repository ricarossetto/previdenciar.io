# previdenciar.io contributor instructions

Read README.md, docs/ARCHITECTURE.md and docs/PRIVACY.md before changing the application.

Preserve exact money, source provenance, pending states and experimental-result labels. Never manufacture missing data or turn a blocked calculation into a financial result. All fixtures must be synthetic.

Do not commit CNIS, personal PDFs, encrypted workspaces, application databases, logs, screenshots of clients, secrets or proprietary binaries. Run the public source review before publishing.

Keep the Tauri application identifier and the legacy storage path compatible unless an explicit, tested migration is implemented. Brand display text does not authorize a storage migration.

PDF preview and export must use the real Rust renderer. Do not replace them with HTML/JSON mocks. Native tests and browser IPC fixtures must be described separately.

The visual identity is already integrated in the v0.16.3 assets (horizontal and square logos, dark/light backgrounds and the Tauri icon). Preserve these assets and verify both themes when changing the UI.

## Português brasileiro e comunicação com o advogado

- Redija todos os textos visíveis (menus, botões, orientações, mensagens, relatórios, PDFs, avisos e respostas de IA) em português brasileiro natural, claro e profissional. Reescreva traduções literais em vez de trocar palavras isoladamente.
- Priorize o vocabulário previdenciário efetivo: CNIS, segurado, vínculo, competência, carência, tempo de contribuição, DER, DIB, RMI, regra de transição, memória de cálculo e pendência documental. Explique siglas na primeira ocorrência quando o público não for especializado.
- Não exponha jargões internos de implementação em telas destinadas ao advogado, como "motor Rust", "adaptador", "entrada tipada", "pipeline", "payload", "hidratar dados", "proveniência" ou "REA". Esses termos podem existir em código, logs e documentação técnica.
- Mensagens devem identificar o problema e a próxima providência: "Confira os vínculos pendentes"; "O cálculo ainda não foi realizado"; "Não foi possível ler o CNIS. Confira se o PDF contém texto selecionável".
- Identifique corretamente o estado do cálculo. "Rascunho", "dados pendentes", "simulação experimental", "cálculo concluído" e "operação bloqueada" têm significados distintos. Não exiba "resultado" quando apenas se registrou um bloqueio.
- Revise ortografia, acentuação, concordância, pontuação e consistência de terminologia entre telas e relatórios. Evite slogans, traduções artificiais, texto excessivamente solene e mensagens repetitivas.
- Não altere identificadores persistidos, contratos IPC, nomes de campos, fórmulas, parâmetros ou resultados numéricos apenas para melhorar a redação. Atualize testes que dependam de rótulos e execute-os.
- Exemplo: "O robô envia uma entrada tipada ao motor Rust" deve ser reescrito como "O sistema usa as informações revisadas para calcular o benefício".
- Exemplo: "Dados conferidos; execução pendente" deve ser reescrito como "Dados revisados. O cálculo ainda não foi realizado".

## Qualidade jurídica do motor

- Antes de classificar uma regra como implementada, confronte seus requisitos, datas, exceções e coeficientes com a legislação oficial. Nunca trate o resultado de outro software como prova suficiente da fórmula.
- Escreva testes com valores exatamente abaixo, iguais e acima dos limites de idade, contribuição, carência e pontuação, inclusive nas viradas de ano.
- A progressão de requisitos da EC 103/2019 depende do ano da DER; não fixe pontuações de 2019 como se fossem permanentes. Tempo especial integra o tempo contributivo e não pode ser somado duas vezes na apuração dos pontos.
- Nas regras especiais, preserve a distinção entre requisito informado pelo operador e requisito efetivamente validado pelos documentos. Se faltarem dados necessários, mantenha o cálculo bloqueado ou explicitamente experimental.
