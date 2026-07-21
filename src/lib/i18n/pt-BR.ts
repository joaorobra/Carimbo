// Brazilian Portuguese UI strings. Keyed identically to `en`; typed against
// StringKey so a missing or stray key fails the build. These mirror the original
// pt-BR copy the app shipped with before it went US-first.

import type { StringKey } from "./en";

export const ptBR: Record<StringKey, string> = {
  // --- app shell / nav ---
  "app.name": "Carimbo",
  "nav.sections": "Seções",
  "nav.snippets": "Snippets",
  "nav.clipboard": "Área de transferência",
  "nav.colors": "Cores",
  "nav.settings": "Configurações",
  "nav.folders": "Pastas",

  // --- window title bar (custom chrome) ---
  "titlebar.minimize": "Minimizar",
  "titlebar.maximize": "Maximizar",
  "titlebar.restore": "Restaurar",
  "titlebar.close": "Fechar",

  // --- snippet list ---
  "snippets.searchPlaceholder": "Buscar snippets…",
  "snippets.searchAria": "Buscar snippets",
  "snippets.newTitle": "Novo snippet (Ctrl+N)",
  "snippets.emptyNoResults": 'Nenhum snippet encontrado para "{query}".',
  "snippets.emptyNone": "Nenhum snippet ainda.",
  "snippets.createFirst": "Criar o primeiro",
  "snippets.placeholderSelect": "Selecione um snippet ou crie um novo.",
  "snippets.favAdd": "Favoritar",
  "snippets.favRemove": "Remover dos favoritos",

  // --- snippet editor ---
  "editor.name": "Nome",
  "editor.namePlaceholder": "Ex.: Meu e-mail",
  "editor.trigger": "Atalho",
  "editor.optional": "(opcional)",
  "editor.triggerPlaceholder": "Ex.: ;email",
  "editor.folder": "Pasta",
  "editor.noFolder": "— Sem pasta —",
  "editor.content": "Conteúdo",
  "editor.insertTokenAria": "Inserir token dinâmico",
  "editor.bodyPlaceholder": "Texto que será inserido…",
  "editor.preview": "Prévia",
  "editor.save": "Salvar",
  "editor.cancel": "Cancelar",
  "editor.delete": "Excluir",
  "editor.confirmDelete": 'Excluir o snippet "{name}"?',
  "editor.errorSave": "Erro ao salvar",
  "editor.errorDuplicateTrigger":
    'O atalho "{trigger}" já está em uso por outro snippet.',
  "editor.richText": "Texto formatado",
  "editor.richToolbar": "Formatação",
  "editor.richNote":
    "A formatação é inserida em aplicativos compatíveis (e-mail, documentos); os demais recebem texto simples. Os tokens dinâmicos continuam funcionando.",
  "editor.bold": "Negrito",
  "editor.italic": "Itálico",
  "editor.underline": "Sublinhado",
  "editor.link": "Adicionar link",
  "editor.linkPrompt": "URL do link:",
  "editor.clearFormat": "Limpar formatação",

  // --- tokens ---
  "token.date.label": "Data",
  "token.date.desc": "Data de hoje",
  "token.time.label": "Hora",
  "token.time.desc": "Hora atual (hh:mm)",
  "token.datetime.label": "Data e hora",
  "token.datetime.desc": "Data e hora",
  "token.clipboard.label": "Área de transferência",
  "token.clipboard.desc": "Conteúdo atual da área de transferência",
  "token.cursor.label": "Cursor",
  "token.cursor.desc":
    "Onde o cursor fica após inserir. Ex.: Prezado {cursor},",
  "token.uuid.label": "UUID",
  "token.uuid.desc": "Um novo identificador único aleatório",
  "token.field.label": "Campo de formulário",
  "token.field.desc":
    "Campo que o usuário preenche ao inserir. Ex.: [[nome_cliente:Nome do cliente]]",

  // --- folders ---
  "folders.all": "Todos",
  "folders.newPlaceholder": "Nome da pasta",
  "folders.new": "Nova pasta",
  "folders.errorCreate": "Erro ao criar pasta",
  "folders.confirmDelete":
    'Excluir a pasta "{name}"? Os snippets não serão apagados.',
  "folders.deleteTitle": "Excluir pasta",
  "folders.deleteAria": "Excluir pasta {name}",

  // --- clipboard history ---
  "clipboard.searchPlaceholder": "Buscar no histórico…",
  "clipboard.searchAria": "Buscar no histórico da área de transferência",
  "clipboard.emptyNoResults": "Nada encontrado no histórico.",
  "clipboard.emptyNone":
    "Seu histórico da área de transferência aparecerá aqui conforme você copia.",
  "clipboard.pin": "Fixar",
  "clipboard.unpin": "Desafixar",
  "clipboard.copy": "Copiar",
  "clipboard.copyAria": "Copiar para a área de transferência",
  "clipboard.delete": "Excluir",
  "clipboard.deleteAria": "Excluir do histórico",
  "clipboard.copied": "Copiado",

  // clip content-type badges
  "clipboard.type.url": "Link",
  "clipboard.type.email": "Email",
  "clipboard.type.color": "Cor",
  "clipboard.type.path": "Caminho",
  "clipboard.type.files": "Arquivos",

  // clip actions
  "clipboard.openUrl": "Abrir link",
  "clipboard.openUrlAria": "Abrir link no navegador",
  "clipboard.email": "Escrever email",
  "clipboard.reveal": "Mostrar na pasta",
  "clipboard.revealAria": "Mostrar arquivo no Explorer",
  "clipboard.promote": "Salvar como atalho",
  "clipboard.promoteAria": "Salvar este item como um atalho reutilizável",
  "clipboard.promoted": 'Atalho "{name}" salvo.',
  "clipboard.moreActions": "Mais ações",
  "clipboard.sourceApp": "Copiado de {app}",
  // ações dinâmicas por tipo de conteúdo no menu de opções da linha
  "clipboard.actions": "Ações",
  "clipboard.pasteHex": "Colar como HEX",
  "clipboard.pasteRgb": "Colar como RGB",
  "clipboard.pasteHsl": "Colar como HSL",
  "clipboard.editImage": "Editar imagem",
  "clipboard.soon": "Em breve",

  // visualização de imagem (lightbox)
  "lightbox.open": "Visualizar imagem",
  "lightbox.close": "Fechar",
  "lightbox.fit": "Ajustar à janela",
  "lightbox.actualSize": "Tamanho real",
  "lightbox.paste": "Colar imagem",
  "lightbox.save": "Mostrar arquivo no Explorer",

  // menu "colar como… / copiar como…"
  "clipboard.transform": "Transformar",
  "clipboard.transformAria": "Copiar com transformação",
  "clipboard.pasteAs": "Colar como…",
  "clipboard.copyAs": "Copiar como…",
  "transform.plain": "Texto simples",
  "transform.upperCase": "MAIÚSCULAS",
  "transform.lowerCase": "minúsculas",
  "transform.titleCase": "Primeira Maiúscula",
  "transform.trim": "Remover espaços",
  "transform.singleLine": "Uma linha só",
  "transform.slug": "forma-slug",
  "transform.base64Encode": "Codificar Base64",
  "transform.base64Decode": "Decodificar Base64",

  // pastas da área de transferência
  "clipboard.allFolders": "Todos",
  "clipboard.moveToFolder": "Mover para pasta",
  "clipboard.removeFromFolder": "— Sem pasta —",
  "clipboard.newFolder": "Nova pasta",
  "clipboard.folderPlaceholder": "Nome da pasta",

  // --- relative time ---
  "time.now": "agora",
  "time.minutes": "{n} min",
  "time.hours": "{n} h",
  "time.days": "{n} d",

  // --- settings ---
  "settings.appearance": "Aparência",
  "settings.theme": "Tema",
  "settings.theme.system": "Automático (SO)",
  "settings.theme.light": "Claro",
  "settings.theme.dark": "Escuro",
  "settings.theme.hcLight": "Alto contraste (claro)",
  "settings.theme.hcDark": "Alto contraste (escuro)",
  "settings.fontSize": "Tamanho da fonte: {pct}%",
  "settings.glassOpacity": "Transparência da paleta: {pct}%",
  "settings.glassOpacityNote":
    "O quão translúcida é a janela de Busca Rápida. Diminua para acentuar o efeito de vidro fosco, ou aumente até 100% para um fundo totalmente sólido.",
  "settings.density": "Densidade",
  "settings.density.compact": "Compacta",
  "settings.density.comfortable": "Confortável",
  "settings.reduceMotion": "Reduzir animações",

  "settings.language": "Idioma e região",
  "settings.uiLanguage": "Idioma da interface",
  "settings.language.en": "Inglês",
  "settings.language.ptBR": "Português (Brasil)",
  "settings.region": "Região",
  "settings.region.us": "Estados Unidos",
  "settings.region.br": "Brasil",
  "settings.region.note":
    "Define o formato de data do {date}: Estados Unidos é mm/dd/aaaa, Brasil é dd/mm/aaaa.",

  "settings.quickSearch": "Busca rápida",
  "settings.globalHotkey": "Atalho global",
  "settings.hotkeyNote":
    "Abre a busca rápida em qualquer aplicativo. Clique no atalho e pressione a nova combinação (ex.: {combo}). Se a combinação já estiver em uso por outro programa, escolha outra.",
  "settings.mainTab": "Aba principal",
  "settings.mainTabNote":
    "Qual aba o atalho abre primeiro. O segundo atalho abre a outra.",
  "settings.opensTab": "abre {tab}",
  "settings.secondHotkey": "Segundo atalho (opcional)",
  "settings.secondHotkeyNote":
    "Defina uma segunda combinação para ir direto para {tab}. Deixe em branco para usar apenas um atalho.",

  "settings.colorHotkey": "Atalho do seletor de cores",
  "settings.colorHotkeyNote":
    "Pressione esta combinação em qualquer aplicativo para capturar uma cor da tela. Depois de clicar, o Carimbo abre na página Cores com a cor carregada. Deixe em branco para desativar.",

  "settings.expansion": "Expansão de atalhos",
  "settings.expansionEnable": "Ativar expansão automática de atalhos",
  "settings.expansionNote":
    "Digite um atalho (ex.: {trigger}) em qualquer aplicativo e ele será substituído pelo texto do snippet. Requer instalar um monitor de teclado — alguns antivírus podem alertar. Fica desativado até você ligar aqui.",
  "settings.injectMethod": "Método de inserção",
  "settings.injectPaste": "Colar (rápido)",
  "settings.injectType": "Digitar (compatível)",
  "settings.injectNote":
    '"Colar" é mais rápido e preserva sua área de transferência. "Digitar" funciona em aplicativos que bloqueiam colar (alguns terminais).',

  "settings.excludedApps": "Não expandir nestes aplicativos",
  "settings.excludedAppsNote":
    "Os atalhos não serão expandidos enquanto estes aplicativos estiverem em foco — útil para gerenciadores de senhas, terminais ou jogos. Informe o nome do executável (ex.: KeePass.exe).",
  "settings.excludedAdd": "Adicionar",
  "settings.excludedPlaceholder": "ex.: KeePass.exe",
  "settings.excludedRemove": "Remover {app}",
  "settings.excludedSuggestions": "Do seu histórico da área de transferência:",

  "settings.clipboard": "Área de transferência",
  "settings.retentionDays": "Manter histórico por: {n} dia{plural}",
  "settings.retentionMax": "Máximo de itens: {n}",
  "settings.retentionNote":
    "Itens fixados e em pastas nunca são removidos automaticamente. Gerenciadores de senha que marcam o conteúdo como sigiloso são ignorados.",

  "settings.backup": "Backup e restauração",
  "settings.backupNote":
    "Salve todos os seus snippets e pastas em um arquivo, ou restaure a partir de um. A importação adiciona à sua biblioteca — nada é sobrescrito ou excluído.",
  "settings.backupExport": "Exportar para arquivo…",
  "settings.backupImport": "Importar de arquivo…",
  "settings.backupExported": "Backup salvo.",
  "settings.backupImported":
    "Importados {snippets} snippet(s) e {folders} pasta(s). {dropped} atalho(s) ignorado(s) (já em uso).",
  "settings.backupError": "Algo deu errado. Tente novamente.",

  "settings.import": "Importar de outro app",
  "settings.importNote":
    "Vindo de outro expansor? Importe seus snippets do espanso (.yml), de um CSV com pares atalho,texto (TextExpander, aText, Beeftext) ou de uma lista JSON. Adiciona à sua biblioteca — nada é sobrescrito.",
  "settings.importButton": "Importar de arquivo…",
  "settings.importDone":
    "Importados {snippets} snippet(s). {dropped} atalho(s) ignorado(s) (já em uso); {skipped} linha(s) sem texto.",
  "settings.importEmpty":
    "Nenhum snippet encontrado no arquivo. Verifique se é uma exportação do espanso, CSV ou JSON.",

  "settings.cloud": "Nuvem e conta",
  "settings.cloudDesc": "Sincronização entre dispositivos e backup na nuvem.",
  "settings.comingSoon": "Em breve",

  // --- hotkey recorder ---
  "hotkey.recordAria": "Gravar atalho da busca rápida",
  "hotkey.press": "Pressione a combinação…",
  "hotkey.restoreDefault": "Restaurar padrão",
  "hotkey.errorSet": "Não foi possível definir este atalho.",
  "hotkey.none": "Não definido",
  "hotkey.clear": "Remover atalho",

  // --- palette ---
  "palette.searchSnippet": "Buscar snippet",
  "palette.searchSnippetPlaceholder": "Buscar snippet…",
  "palette.searchClipboard": "Buscar na área de transferência",
  "palette.searchClipboardPlaceholder": "Buscar no histórico…",
  "palette.emptyNoSnippets": "Nenhum snippet encontrado",
  "palette.emptyNoSnippetsYet": "Nenhum snippet ainda",
  "palette.emptyNoClips": "Nada encontrado no histórico",
  "palette.emptyNoClipsYet": "Seu histórico aparecerá aqui",
  "palette.emptyNoSnippetsHint": "Crie snippets no Carimbo e insira-os aqui.",
  "palette.emptyNoClipsHint": "Copie algo e aparecerá aqui.",
  "palette.emptyNoResultsHint": "Tente outra busca.",
  "palette.insertError": "Não foi possível inserir — tente novamente.",
  "palette.navigate": "navegar",
  "palette.switchTab": "trocar aba",
  "palette.insert": "inserir",
  "palette.paste": "colar",
  "palette.close": "fechar",
  "palette.results": "{n} resultado{plural}",
  "palette.pinAdd": "Fixar nos favoritos",
  "palette.pinRemove": "Desafixar dos favoritos",
  "palette.moreOptions": "Mais opções",
  "palette.insertAs": "Inserir como…",
  "palette.pasteAs": "Colar como…",
  // Cabeçalhos de seção (sem busca ativa) quando os favoritos vão para o topo.
  "palette.favorites": "Favoritos",
  "palette.otherSnippets": "Outros",

  // --- variable form ---
  "form.fillFields": "Preencher campos",
  "form.back": "Voltar (Esc)",
  "form.backAria": "Voltar à lista",
  "form.fieldPlaceholder": "Digite {label}…",
  "form.nextField": "próximo campo",
  "form.insert": "inserir",
  "form.backHint": "voltar",
  "form.insertBtn": "Inserir",

  // --- radial ---
  "radial.title": "Vários gatilhos parecidos",
  "radial.chooseAria": "Escolher snippet",
  "radial.navigate": "navegar",
  "radial.choose": "escolher",
  "radial.insert": "inserir",
  "radial.close": "fechar",

  // --- color picker ---
  "colors.title": "Cores",
  "colors.pick": "Capturar da tela",
  "colors.pickAria": "Capturar uma cor da tela",
  "colors.picking": "Clique em qualquer lugar para capturar uma cor…",
  "colors.overlayHint": "Clique para capturar · Esc cancela",
  "colors.edit": "Ajustar",
  "colors.hex": "Hex",
  "colors.red": "Vermelho",
  "colors.green": "Verde",
  "colors.blue": "Azul",
  "colors.hue": "Matiz",
  "colors.saturation": "Saturação",
  "colors.lightness": "Luminosidade",
  "colors.scrollAdjust": "Rolar para ajustar",
  "colors.scrollAdjustNote":
    "Passe o mouse sobre um campo ou controle e gire a roda para alterar em 1 (segure Shift para 10).",
  "colors.tones": "Tons",
  "colors.darker": "Mais escuro",
  "colors.lighter": "Mais claro",
  "colors.baseTone": "Cor atual",
  "colors.toneAria": "Selecionar tom {value}",
  "colors.copyFormats": "Copiar como",
  "colors.copyAria": "Copiar valor {format}",
  "colors.copied": "{value} copiado.",
  "colors.copyError": "Não foi possível copiar — tente novamente.",
  "colors.recent": "Recentes",
  "colors.recentAria": "Usar cor recente {value}",

  // --- toasts ---
  "toast.elevatedBlocked":
    "Não é possível expandir em um aplicativo executado como administrador.",

  // --- onboarding ---
  "onboarding.welcomeAria": "Bem-vindo ao Carimbo",
  "onboarding.skipAria": "Pular introdução",
  "onboarding.title": "Bem-vindo ao Carimbo",
  "onboarding.lead":
    "Guarde textos que você usa sempre — nomes, CPF, endereços, assinaturas — e insira em qualquer lugar num instante. Vamos criar o seu primeiro.",
  "onboarding.name": "Nome",
  "onboarding.namePlaceholder": "Ex.: Meu e-mail",
  "onboarding.text": "Texto",
  "onboarding.textPlaceholder": "Ex.: voce@exemplo.com",
  "onboarding.trigger": "Atalho",
  "onboarding.triggerPlaceholder": "Ex.: ;email",
  "onboarding.tip": "Pressione {combo} em qualquer lugar para buscar e inserir.",
  "onboarding.skip": "Pular",
  "onboarding.create": "Criar meu primeiro snippet",
  "onboarding.error": "Não foi possível criar o snippet. Tente outro atalho.",

  // --- first-run region picker ---
  "region.title": "Bem-vindo ao Carimbo",
  "region.lead":
    "Onde você vai usar o Carimbo? Isso define o formato de data e os snippets de exemplo com que você começa. Você pode mudar depois nas Configurações.",
  "region.us": "Estados Unidos",
  "region.usDesc": "Interface em inglês · datas mm/dd/aaaa",
  "region.br": "Brasil",
  "region.brDesc": "Interface em português · datas dd/mm/aaaa",
  "region.continue": "Continuar",
};
