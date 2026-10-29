# Glosario español (es)

Base: `src/lib/i18n/en.ts` (claves), tono y sentido — `ru.ts` (voz original de la autora).
Regla del proyecto (CLAUDE.md § 1.1): nombres directos, sin metáforas; una cosa — una palabra en todas partes
(juego, mesa del admin, estudio, documentos).

## Decisiones generales

| Tema | Decisión | Por qué |
|---|---|---|
| Tratamiento | **tú** (informal), en todo el sitio | El sitio habla al visitante en tono cercano y tranquilo, como la autora en ruso/inglés. *Usted* sonaría a tienda o a banco |
| Variante | Español peninsular neutro, sin *vosotros* | Una persona a la vez («tú»), así que el plural casi no aparece; si aparece, «ustedes» no se usa: se reformula |
| Comillas | «» (y “” dentro) | Acuerdo común del plan (es, it) |
| Tono | Sobrio, sin presión de venta, sin «¡…!» si el original no lo tiene | § 1: no es una tienda |
| Longitud | Tan corto como el inglés en botones, sellos y caras de carta | El espacio de la interfaz es estrecho |
| Nombres propios | Ritunia, Telegram, Google, Apple, Pinterest — sin cambios | |
| Marcadores | `{brand}`, `{n}`, `{name}`… y etiquetas HTML intactos | |

## Secciones del sitio

| en | es | Nota |
|---|---|---|
| Works / Archive | **Obras** / **Archivo** | «Obra» = pieza de la autora; «archivo» = la lista `/figurines` |
| work, piece, figure | obra, pieza, figura | «figura» para el objeto físico |
| Gazette | **Gaceta** | |
| leaf (of the gazette) | **hoja** | Se mantiene: nombre fijo del proyecto para una entrada de la gaceta |
| Tales / tall tale | **Cuentos** / **cuento** | «Небылица/байка» = relato breve inventado; «cuento» es la palabra común y clara |
| Game | **Juego** | |
| Workshop | **Taller** | |
| Studio | **Estudio** | |
| Author | **Autora** | La autora es una mujer (Ritunia) |
| cabinet (the site as a collection) | **gabinete** | Ya fijado en en.ts («Cabinet of Gothic Miniatures» → «Gabinete de miniaturas góticas») |
| keeper | la autora del sitio → **la autora** (estudio, aprobación, búsqueda); el contrincante en el juego → **el rival** | § 1.1 / § 9.11: sin «guardián». El buscador «keeper» de la portada se dice simplemente «búsqueda» |
| Ask the site | **Pregunta al sitio** | |

## Estados de una obra

| en | es |
|---|---|
| Available | **Disponible** |
| Lost (sold) | **Cedida** (lista: «Cedidas») | Decisión del coordinador: la obra salió de las manos de la autora, sin vocabulario de venta. «Perdida» se leía como «extraviada». También en la ficha guardada del perfil y en el admin |
| Reserved | **Reservada** |
| In the making (in_progress) | **En proceso** |

## Reservas y solicitudes

| en | es |
|---|---|
| booking | **reserva** (de fechas de visita) |
| showing | **exposición** / visita |
| request | **solicitud** |
| waitlist | **lista de espera** |
| cancel token / receipt | **código de cancelación** / resguardo |
| commission | **encargo** |
| notify me | **avísame** |

## Juego (§ 9.11)

| en (código) | es | Nota |
|---|---|---|
| card | **carta** | |
| rank (`tier`) | **rango** | Nunca «nivel» para el rango |
| level (`level`) | **nivel** | Estado de una copia concreta, se sube con polvo |
| dust | **polvo** | Nombre fijo del proyecto |
| feed / grain | **grano** | En ruso «зерно»; el texto legal en inglés ya dice «grain». «raven feed» → «grano de cuervo» |
| health | **salud** | |
| mana | **maná** | |
| armour | **armadura** | |
| power | **fuerza** | |
| Spell defence (`ward`) | **Defensa mágica** | Par de «Armadura»: protege del otro tipo de daño |
| Immune (`Numb`) | **Inmune** (estado) / **Inmunidad** (acción) | |
| Dominated (`Swayed`) / Dominate | **Dominada** (estado) / **Dominar** (acción) | |
| Hidden (`Veiled`) | **Invisible**; verbo «Hacer invisible» | No se puede elegir como objetivo |
| Steadfast (`Rested`) | **Respiro** | ru «Передышка»: durante él no se puede volver a inmovilizar |
| Guarding | **Guardia** (estado); verbo «Guardia», objetivo «A quién cubrir» | ru «Заслон». «Escudo» ya es el hechizo Shield (`battlesVerbShield`), por eso no |
| damage type | **tipo de daño**; «contra golpes» / «contra hechizos» | |
| physical / magic | físico / mágico | |
| race | **raza** | |
| trait | **rasgo** | |
| ability | **habilidad** | |
| spell | **hechizo** | |
| strike / blow | **golpe** | |
| counter-strike | contragolpe | |
| étude (en.ts: «Studies») | **desafío** / **Desafíos** | Decisión del coordinador: «Estudios» chocaba con «Estudio» (la sección Studio), § 1.1. En la lista y los botones se dice «batalla» cuando se habla de una partida concreta |
| errand (en.ts: «Tasks») | **misión** / Misiones | «Encargo» ya es commission |
| shelf (`/battles`) | **estante** | Nombre fijo del proyecto (полка) |
| deck | **mazo** | |
| hand | **mano** | |
| lineup | **alineación** | |
| round | **ronda** | |
| turn | **turno** | |
| structure | **construcción** | Muro, torre: no es una tropa |
| forecast (`foresee`) | **pronóstico** | |
| take sheet (лист взятия) | hoja de la carta | Nombre fijo |
| card face (Лицо карты) | **Cara de la carta** | Nombre fijo |
| battle cell (клетка боя) | **casilla** | |
| breakthrough | **avance** | |
| Benefit (род земли «Выгода») | **Ventaja** | |
| Danger / Forbidden | Peligro / Prohibido | |

### Terreno

| en | es |
|---|---|
| wall | **muro** |
| ravine | **barranco** |
| pit | **foso** |
| mire | **ciénaga** |
| cover | **parapeto** |
| hill | **colina** |
| spring | **manantial** |
| embers | **brasas** |
| altar | **altar** |
| ice | **hielo** |
| storm | **tormenta** |
| grave | **tumba** |
| crystal | **cristal** |
| stained glass | **vidriera** |
| rift | **grieta** |
| maelstrom | **remolino** |

## Entrada (login)

| en | es | Nota |
|---|---|---|
| signs (visual password) | **signos** | Cuatro signos elegidos: «signo — 1 de 4» |
| key seal | sello de la clave | |
| one-time word (Telegram) | **palabra** de un solo uso | |
| sign in / sign out | entrar / salir | |
| account | cuenta | |

## Cuentos

| en | es |
|---|---|
| read (mark) | **leído** |
| sequel | **continuación** |
| "Want a sequel" | «Quiero la continuación» |
| reading time | X min de lectura |
| postcard (from a phrase) | **postal** |
| Book of the house (newsletter) | **lista de correo** / «Libro de la casa» donde es nombre fijo |

### Otros estados y canales

| en | es |
|---|---|
| Bound (`Hold` оцепенение) / Stun (verbo `battlesVerbControl`) | **Inmovilizada** (estado) / **Inmovilizar** (acción); objetivo «A quién inmovilizar» | Una cosa, una palabra: antes el verbo decía «Aturdir». Igual en es.ts y es.admin.ts |
| Silenced | Silenciada |
| Disarmed | Desarmada |
| Thorns | Espinas |
| Festering | Podredumbre |
| Knitting | Regeneración |
| Body (tipo de carta `unit`) | Criatura |
| Spell / Relic / Structure (tipos) | Hechizo / Reliquia / Construcción |
| Bodily / Charmed / Evil eye (canal) | Físico / Mágico / Mal de ojo |
| body (carta en el campo, en textos) | carta (§ 9.11: no «cuerpo») |
| reach / step / speed / mend | alcance / paso / velocidad / curación |
| cooldown | recarga |
| Barrier / Hazard / Boon (clase de terreno) | Barrera / Peligro / Ventaja |
| frame (estudio) | marco |
| licence | licencia |
| season | temporada |

## Añadido al traducir los datos (G-es)

| ru / en | es | Nota |
|---|---|---|
| Люди / People (raza) | **Humanos** | |
| Водяные / Water Folk (raza) | **Gente del agua** | |
| Нежить / Undead (raza) | **No muertos** | |
| Гномы / Ведьмы / Маги | Gnomos / Brujas / Magos | |
| Скромная · Крепкая · Памятная · Редкая · Эпическая (rangos de marco) | Modesta · Robusta · Memorable · Rara · Épica | |
| Строение / Осада (tipo de carta) | Construcción / Asedio | |
| пирожок (cuentos) | empanadilla | |

## Obras: materiales y técnica (añadido por G-es)

| en | es | Nota |
|---|---|---|
| polymer clay | **arcilla polimérica** | |
| acrylic paint / acrylics | **pintura acrílica** / **acrílicos** | |
| wire armature | **armazón de alambre** | |
| art doll / author's doll | **muñeca artística** / **muñeca de autor** | |
| OOAK, one of a kind | **pieza única** | La sigla no se usa en el texto español |
| hand sculpted / hand painted | **modelada a mano** / **pintada a mano** | Campo «técnica»: «Modelado a mano, pintado a mano, hecho a mano» (ver abajo, R-es) |
| poseable | **articulable** | |
| faux fur / faux leather | piel sintética / polipiel | |
| curio cabinet / cabinet of curiosities | gabinete de curiosidades | |
| Baba Yaga | **Baba Yagá** | |

## Añadido al traducir la administración (F-es)

| en / ru | es | Nota |
|---|---|---|
| étude / этюд | **desafío** / **Desafíos** | Decisión del coordinador: no «Estudios» (choca con «Estudio»). Usado en `es.admin.ts` y `es.ts` |
| Lost (`sold`) | **Cedida** | Decisión del coordinador, en lugar de «Perdida» |
| meeting (`playerSide: deck`) | **encuentro** | Combate con el mazo propio del visitante |
| keeper (lado del juego, «house won») | **rival** | § 9.11 |
| house rules (`Rules::default`) | **reglas generales** | |
| bench (mesa de prueba del admin) | **Mesa de prueba** | |
| By hand (pestaña) | **A mano** | Grano dado a mano |
| preset / saved frame / wardrobe | **marco guardado** | |
| sheet (lámina de recursos) | **lámina** | Distinto de «hoja de la carta» |
| asset / part | **recurso** / **pieza** | |
| leaf / note (gazette, admin) | **hoja** | |
| House book (newsletter) | **lista de correo** | Igual que `es.ts` |
| similar works | **obras parecidas** | |
| Unfinished (pestaña) | **Sin terminar** | |

## Decisiones de la revisión (R-es)

| Tema | Decisión | Por qué |
|---|---|---|
| letter / письмо (email) | **correo** (un correo, los correos); nunca «carta» | «Carta» ya es la carta del juego: una palabra para dos cosas (§ 1.1). «Mensaje» para lo que el visitante escribe a la autora |
| имя (= cuenta de usuario) | **cuenta**; «usuario» cuando hay que distinguirla de la cuenta de Google/Telegram | «Nombre» como cuenta necesita explicación; es una imagen |
| tales / небылицы | **cuentos**, también en textos del juego («los cuentos que lees»), no «historias» | Una cosa, una palabra |
| Tratamiento en los cuentos | Los narradores pueden hablar a los lectores con *vosotros* («os voy a confesar») y la abuela de las empanadillas trata de *usted* al cliente | Es la voz del personaje, no la interfaz; en la interfaz sigue siendo *tú* |

### Nombres: carta, obra y cuento deben coincidir

| Personaje (slug de la carta / obra) | Carta | Obra (nombre) | Cuento |
|---|---|---|---|
| forest-gnome / small-doll-gnome (Лесовик) | **Duende del bosque** | Pequeño gnomo | — (en «Curo por criptografía…» el gurú es «el duende del bosque», mismo ser folclórico) |
| small-art-doll-forester (The Forester) | — | Pequeña muñeca artística: **guardabosques** | «El guardabosques y su memoria inolvidable» |
| falconer / small-art-doll-falconer | **Cetrero** | Pequeña muñeca artística: cetrero | — |
| the-creature / Frankenstein | **El Cosido** | la criatura de Frankenstein | — (prueba de golpe en la portada: «el Cosido») |
| yaga-with-a-bird | **Pequeña Yagá** | Pequeña Baba Yagá con pájaro | — (sugerencias de búsqueda: «Pequeña Yagá», «Baba Yagá») |
| captain-salazar | **Capitán Salazar** | capitán Salazar | «Cumpliré todos tus deseos» (figura) |
| piper / bagpiper | **El Flautista** / **Gaitero de la montaña** | el flautista de Hamelín / gaitero | — |
| gnome-axeman | **Hachero** | gnomo con hacha | «Matar, despedazar y luego perdonar» (figura) |

### Materiales y técnica (actualizado)

- Materiales: siempre **pintura acrílica** (también donde el original dice «acrylics»): el filtro del archivo agrupa por valor.
- Técnica: **«Modelado a mano, pintado a mano, hecho a mano»** (antes «…pintura a mano, confección a mano», poco natural).

## Compra por Etsy y solicitudes (octubre 2026)

| ru | es |
|---|---|
| Приобрести (botón; lleva a la página de la obra en Etsy) | **Comprar** |
| Хочу похожую | **Quiero una parecida** |
| Заявка (sin compromiso) | **solicitud** |
| Заказать фигурку | **Encargar una figura** |
| Почему оплата через Etsy | **Por qué el pago se hace en Etsy** |
| В наличии | **Disponible** |
