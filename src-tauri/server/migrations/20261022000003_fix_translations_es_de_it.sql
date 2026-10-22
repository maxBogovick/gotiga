-- Corrections to the Spanish, German and Italian work translations of
-- 20261020000003 (already applied on some databases, so it cannot change):
-- the fixed English source (tools/i18n/fixes/README.md) — a dog, not a rat;
-- a gnome, not a troll; a colourful backpack full of brushwood.
-- A field is changed only while it still holds exactly the text 20261020000003
-- wrote, so a translation the keeper has edited since is left alone.

-- small-art-doll-gnome-with-axe (de): short_text
UPDATE figurine_translations SET short_text = $fx$Kleine Künstlerpuppe: kleiner Gnom mit Axt. Aus Polymerton, mit Acrylfarben bemalt und von Hand angekleidet. Eine vollständig handgefertigte Gnomfigur, etwa 8 cm hoch$fx$
 WHERE lang = 'de' AND short_text = $fx$Kleine Künstlerpuppe: kleiner Gnom mit Axt. Aus Polymerton, mit Acrylfarben bemalt und von Hand angekleidet. Eine vollständig handgefertigte Trollfigur, etwa 8 cm hoch$fx$
   AND figurine_id = (SELECT id FROM figurines WHERE slug = 'small-art-doll-gnome-with-axe');

-- small-art-doll-gnome-with-axe (es): short_text
UPDATE figurine_translations SET short_text = $fx$Pequeña muñeca de autor: un gnomito con hacha. Hecha en arcilla polimérica, pintada con acrílicos y vestida a mano. Figura de gnomo hecha por completo a mano, de unos 8 cm de altura$fx$
 WHERE lang = 'es' AND short_text = $fx$Pequeña muñeca de autor: un gnomito con hacha. Hecha en arcilla polimérica, pintada con acrílicos y vestida a mano. Figura de trol hecha por completo a mano, de unos 8 cm de altura$fx$
   AND figurine_id = (SELECT id FROM figurines WHERE slug = 'small-art-doll-gnome-with-axe');

-- small-art-doll-gnome-with-axe (it): short_text
UPDATE figurine_translations SET short_text = $fx$Piccola bambola d’autore: un piccolo gnomo con l’ascia. In argilla polimerica, dipinta con colori acrilici e vestita a mano. Figura di gnomo interamente fatta a mano, alta circa 8 cm$fx$
 WHERE lang = 'it' AND short_text = $fx$Piccola bambola d’autore: un piccolo gnomo con l’ascia. In argilla polimerica, dipinta con colori acrilici e vestita a mano. Figura di troll interamente fatta a mano, alta circa 8 cm$fx$
   AND figurine_id = (SELECT id FROM figurines WHERE slug = 'small-art-doll-gnome-with-axe');

-- small-authors-doll-man-with-the-dog (de): full_description
UPDATE figurine_translations SET full_description = $fx$Alter Mann mit Hund – handgefertigte Künstlerpuppe, ein Unikat
(etwa 15–16 cm hoch / rund 6–6,3 Zoll)

Diese Künstlerpuppe ist ein Unikat, vollständig von Hand aus Polymerton modelliert und sorgfältig mit hochwertigen Acrylfarben bemalt. Sie zeigt einen nachdenklichen alten Mann mit seinem treuen Hund als Gefährten und fängt einen stillen Augenblick der Freundschaft, des Vertrauens und der Verbundenheit ein.

Jedes Detail ist von Hand gefertigt, vom ausdrucksvollen Gesicht mit den vom Wetter gezeichneten Zügen und dem langen grauen Bart bis zur handgenähten Kleidung und dem winzigen tierischen Freund. Der nachdenkliche Ausdruck und der sanfte Umgang mit dem Hund deuten eine Geschichte an, die darauf wartet, erzählt zu werden, und laden dazu ein, sich sein Leben, seine Reisen und Abenteuer auszumalen.

Dank eines Drahtgerüsts sind die Glieder beweglich; die Puppe trägt handgenähte Kleidung, die eigens für diese Gestalt geschneidert wurde. Bei der Herstellung werden keine Formen verwendet, und so ist sie ein echtes Unikat, das sich nie genau wiederholen lässt.

Für Sammler von Künstlerpuppen, Fantasy-Gestalten, Volkskunst und literarisch inspirierten Miniaturen bringt dieses Stück Wärme, Persönlichkeit und erzählerischen Reiz in jede Vitrine. Ob im Bücherregal, in einer Kuriositätenvitrine oder unter anderen einzigartigen Handarbeiten – es zieht Blicke auf sich und weckt Neugier.

Die Puppe ist etwa 15–16 cm (6–6,3 Zoll) hoch. Sie wird sorgfältig in schützender Verpackung verpackt und mit Sendungsnummer verschickt. Bitte behutsam behandeln: Sie ist ein zartes Sammlerstück, gedacht zum Betrachten und nicht zum Spielen.$fx$
 WHERE lang = 'de' AND full_description = $fx$Alter Mann mit Hund – handgefertigte Künstlerpuppe, ein Unikat
(etwa 15–16 cm hoch / rund 6–6,3 Zoll)

Diese Künstlerpuppe ist ein Unikat, vollständig von Hand aus Polymerton modelliert und sorgfältig mit hochwertigen Acrylfarben bemalt. Sie zeigt einen nachdenklichen alten Mann mit seiner treuen Ratte als Gefährtin und fängt einen stillen Augenblick der Freundschaft, des Vertrauens und der Verbundenheit ein.

Jedes Detail ist von Hand gefertigt, vom ausdrucksvollen Gesicht mit den vom Wetter gezeichneten Zügen und dem langen grauen Bart bis zur handgenähten Kleidung und dem winzigen tierischen Freund. Der nachdenkliche Ausdruck und der sanfte Umgang mit der Ratte deuten eine Geschichte an, die darauf wartet, erzählt zu werden, und laden dazu ein, sich sein Leben, seine Reisen und Abenteuer auszumalen.

Dank eines Drahtgerüsts sind die Glieder beweglich; die Puppe trägt handgenähte Kleidung, die eigens für diese Gestalt geschneidert wurde. Bei der Herstellung werden keine Formen verwendet, und so ist sie ein echtes Unikat, das sich nie genau wiederholen lässt.

Für Sammler von Künstlerpuppen, Fantasy-Gestalten, Volkskunst und literarisch inspirierten Miniaturen bringt dieses Stück Wärme, Persönlichkeit und erzählerischen Reiz in jede Vitrine. Ob im Bücherregal, in einer Kuriositätenvitrine oder unter anderen einzigartigen Handarbeiten – es zieht Blicke auf sich und weckt Neugier.

Die Puppe ist etwa 15–16 cm (6–6,3 Zoll) hoch. Sie wird sorgfältig in schützender Verpackung verpackt und mit Sendungsnummer verschickt. Bitte behutsam behandeln: Sie ist ein zartes Sammlerstück, gedacht zum Betrachten und nicht zum Spielen.$fx$
   AND figurine_id = (SELECT id FROM figurines WHERE slug = 'small-authors-doll-man-with-the-dog');

-- small-authors-doll-man-with-the-dog (es): full_description
UPDATE figurine_translations SET full_description = $fx$Anciano con perro, pieza única – muñeca artística hecha a mano
(unos 15–16 cm de altura)

Esta muñeca artística única está modelada por completo a mano en arcilla polimérica y pintada a mano con cuidado con acrílicos de alta calidad. Representa a un anciano pensativo acompañado de su fiel perro, y recoge un momento tranquilo de amistad, confianza y compañía.

Cada detalle está hecho a mano, desde su rostro expresivo, sus rasgos curtidos y su larga barba gris hasta la ropa hecha a mano y su diminuto amigo animal. La expresión meditabunda del personaje y su trato delicado con el perro sugieren una historia por contar e invitan a imaginar su vida, sus viajes y sus aventuras.

Gracias a un armazón de alambre, la muñeca tiene extremidades móviles y viste ropa hecha a mano, confeccionada especialmente para este personaje. No se usan moldes en el proceso, así que es una verdadera pieza única de colección que nunca podrá duplicarse exactamente.

Ideal para coleccionistas de muñecas artísticas, personajes de fantasía, arte popular y miniaturas de inspiración literaria, esta pieza aporta calidez, carácter y el encanto de una historia a cualquier vitrina. Ya sea en una estantería de libros, en un gabinete de curiosidades o entre una colección de creaciones únicas hechas a mano, seguro que llama la atención y despierta la curiosidad.

La muñeca mide unos 15–16 cm de altura. Se embalará con cuidado en un envoltorio protector y se enviará con número de seguimiento. Conviene manipularla con cuidado: es una pieza artística delicada, pensada para exponerse y no para jugar.$fx$
 WHERE lang = 'es' AND full_description = $fx$Anciano con perro, pieza única – muñeca artística hecha a mano
(unos 15–16 cm de altura)

Esta muñeca artística única está modelada por completo a mano en arcilla polimérica y pintada a mano con cuidado con acrílicos de alta calidad. Representa a un anciano pensativo acompañado de su fiel rata, y recoge un momento tranquilo de amistad, confianza y compañía.

Cada detalle está hecho a mano, desde su rostro expresivo, sus rasgos curtidos y su larga barba gris hasta la ropa hecha a mano y su diminuto amigo animal. La expresión meditabunda del personaje y su trato delicado con la rata sugieren una historia por contar e invitan a imaginar su vida, sus viajes y sus aventuras.

Gracias a un armazón de alambre, la muñeca tiene extremidades móviles y viste ropa hecha a mano, confeccionada especialmente para este personaje. No se usan moldes en el proceso, así que es una verdadera pieza única de colección que nunca podrá duplicarse exactamente.

Ideal para coleccionistas de muñecas artísticas, personajes de fantasía, arte popular y miniaturas de inspiración literaria, esta pieza aporta calidez, carácter y el encanto de una historia a cualquier vitrina. Ya sea en una estantería de libros, en un gabinete de curiosidades o entre una colección de creaciones únicas hechas a mano, seguro que llama la atención y despierta la curiosidad.

La muñeca mide unos 15–16 cm de altura. Se embalará con cuidado en un envoltorio protector y se enviará con número de seguimiento. Conviene manipularla con cuidado: es una pieza artística delicada, pensada para exponerse y no para jugar.$fx$
   AND figurine_id = (SELECT id FROM figurines WHERE slug = 'small-authors-doll-man-with-the-dog');

-- small-authors-doll-man-with-the-dog (it): full_description
UPDATE figurine_translations SET full_description = $fx$Vecchio con il cane, pezzo unico – bambola d’artista fatta a mano
(alta circa 15–16 cm)

Questa bambola d’artista in un unico esemplare è interamente modellata a mano in argilla polimerica e dipinta con cura con colori acrilici di alta qualità. Ritrae un vecchio pensieroso accompagnato dal suo fedele compagno, un cane, e coglie un momento quieto di amicizia, di fiducia e di compagnia.

Ogni dettaglio è realizzato a mano, dal viso espressivo, dai tratti segnati dagli anni e dalla lunga barba grigia agli abiti cuciti a mano e al minuscolo amico animale. L’espressione assorta del personaggio e il gesto gentile verso il cane suggeriscono una storia che aspetta di essere raccontata, e invitano a immaginare la sua vita, i suoi viaggi e le sue avventure.

Costruita su un’armatura di filo metallico, la bambola ha arti mobili ed è vestita con abiti fatti a mano apposta per questo personaggio. Nella lavorazione non si usano stampi: è un vero pezzo unico da collezione, che non potrà mai essere riprodotto identico.

Pensata per chi colleziona bambole d’artista, personaggi fantastici, arte popolare e miniature ispirate alla letteratura, porta calore, carattere e il fascino del racconto in qualsiasi esposizione. Su uno scaffale di libri, in una vetrina di curiosità o tra altre creazioni fatte a mano, attira lo sguardo e risveglia la curiosità.

La bambola è alta circa 15–16 cm. Viene imballata con cura in una confezione protettiva e spedita con numero di tracciamento. Va maneggiata con cura: è un delicato oggetto d’arte da collezione, pensato per essere esposto e non per giocare.$fx$
 WHERE lang = 'it' AND full_description = $fx$Vecchio con il cane, pezzo unico – bambola d’artista fatta a mano
(alta circa 15–16 cm)

Questa bambola d’artista in un unico esemplare è interamente modellata a mano in argilla polimerica e dipinta con cura con colori acrilici di alta qualità. Ritrae un vecchio pensieroso accompagnato dal suo fedele compagno, un topo, e coglie un momento quieto di amicizia, di fiducia e di compagnia.

Ogni dettaglio è realizzato a mano, dal viso espressivo, dai tratti segnati dagli anni e dalla lunga barba grigia agli abiti cuciti a mano e al minuscolo amico animale. L’espressione assorta del personaggio e il gesto gentile verso il topo suggeriscono una storia che aspetta di essere raccontata, e invitano a immaginare la sua vita, i suoi viaggi e le sue avventure.

Costruita su un’armatura di filo metallico, la bambola ha arti mobili ed è vestita con abiti fatti a mano apposta per questo personaggio. Nella lavorazione non si usano stampi: è un vero pezzo unico da collezione, che non potrà mai essere riprodotto identico.

Pensata per chi colleziona bambole d’artista, personaggi fantastici, arte popolare e miniature ispirate alla letteratura, porta calore, carattere e il fascino del racconto in qualsiasi esposizione. Su uno scaffale di libri, in una vetrina di curiosità o tra altre creazioni fatte a mano, attira lo sguardo e risveglia la curiosità.

La bambola è alta circa 15–16 cm. Viene imballata con cura in una confezione protettiva e spedita con numero di tracciamento. Va maneggiata con cura: è un delicato oggetto d’arte da collezione, pensato per essere esposto e non per giocare.$fx$
   AND figurine_id = (SELECT id FROM figurines WHERE slug = 'small-authors-doll-man-with-the-dog');

-- small-doll-gnome-with-fancy-bag (de): short_text
UPDATE figurine_translations SET short_text = $fx$Kleine Puppe: Gnom mit einem bunten Rucksack voller Reisig. Aus Polymerton, mit Acrylfarben bemalt und von Hand angekleidet. Eine vollständig handgefertigte Figur, ein Unikat.$fx$
 WHERE lang = 'de' AND short_text = $fx$Kleine Puppe mit einem Bündel Reisig. Aus Polymerton, mit Acrylfarben bemalt und von Hand angekleidet. Eine vollständig handgefertigte Figur, ein Unikat.$fx$
   AND figurine_id = (SELECT id FROM figurines WHERE slug = 'small-doll-gnome-with-fancy-bag');

-- small-doll-gnome-with-fancy-bag (de): full_description
UPDATE figurine_translations SET full_description = $fx$Darf ich vorstellen: ein neugieriger kleiner Waldwanderer in schlichter handgenähter Kleidung, mit einem großen bunten Rucksack voller Reisig. Mit seinem hohen schiefen Hut, dem ausdrucksvollen Gesicht, dem leicht zerzausten Haar und den langen spitzen Schuhen sieht er aus, als wandere er schon eine ganze Weile durch den Wald – und habe wohl die eine oder andere Geschichte zu erzählen.

Der übergroße Rucksack ist aus strukturierten Stoffen genäht und mit bunten gewebten Mustern verziert, ein schöner Gegensatz zu seiner schlichten dunklen Kleidung. Er trägt einen naturbelassenen Wanderstock aus Holz und wirkt wie ein Reisender aus alter Zeit, der jederzeit bereit ist, den nächsten Waldpfad einzuschlagen.

Die Puppe ist vollständig von Hand modelliert und bemalt, jedes Detail einzeln gefertigt. Ein Drahtgerüst im Inneren erlaubt es, Arme und Beine leicht zu bewegen und die Haltung zu verändern. Die schlichte Kleidung, der hohe handgemachte Hut, der strukturierte Rucksack, die winzigen Knöpfe, die spitzen Schuhe und der Stab aus Naturholz geben ihm seinen ganz eigenen altweltlichen Charakter.

Mit etwa 10–11 cm Höhe ist er eine schöne Ergänzung für eine Fantasy- oder Waldsammlung, eine Miniaturvitrine, ein Diorama, eine Puppenhausszene oder einfach einen besonderen Platz im Regal.

Er ist eine echte handgefertigte Künstlerpuppe, ein Unikat – eine zweite genau wie ihn wird es nie geben.$fx$
 WHERE lang = 'de' AND full_description = $fx$Darf ich vorstellen: ein neugieriger kleiner Waldwanderer in schlichter handgenähter Kleidung, mit einem großen bunten Rucksack voller geheimnisvoller Schätze. Mit seinem hohen schiefen Hut, dem ausdrucksvollen Gesicht, dem leicht zerzausten Haar und den langen spitzen Schuhen sieht er aus, als wandere er schon eine ganze Weile durch den Wald – und habe wohl die eine oder andere Geschichte zu erzählen.

Der übergroße Rucksack ist aus strukturierten Stoffen genäht und mit bunten gewebten Mustern verziert, ein schöner Gegensatz zu seiner schlichten dunklen Kleidung. Er trägt einen naturbelassenen Wanderstock aus Holz und wirkt wie ein Reisender aus alter Zeit, der jederzeit bereit ist, den nächsten Waldpfad einzuschlagen.

Die Puppe ist vollständig von Hand modelliert und bemalt, jedes Detail einzeln gefertigt. Ein Drahtgerüst im Inneren erlaubt es, Arme und Beine leicht zu bewegen und die Haltung zu verändern. Die schlichte Kleidung, der hohe handgemachte Hut, der strukturierte Rucksack, die winzigen Knöpfe, die spitzen Schuhe und der Stab aus Naturholz geben ihm seinen ganz eigenen altweltlichen Charakter.

Mit etwa 10–11 cm Höhe ist er eine schöne Ergänzung für eine Fantasy- oder Waldsammlung, eine Miniaturvitrine, ein Diorama, eine Puppenhausszene oder einfach einen besonderen Platz im Regal.

Er ist eine echte handgefertigte Künstlerpuppe, ein Unikat – eine zweite genau wie ihn wird es nie geben.$fx$
   AND figurine_id = (SELECT id FROM figurines WHERE slug = 'small-doll-gnome-with-fancy-bag');

-- small-doll-gnome-with-fancy-bag (es): short_text
UPDATE figurine_translations SET short_text = $fx$Pequeña muñeca: gnomo con una mochila de colores llena de leña. Hecha en arcilla polimérica, pintada con acrílicos y vestida a mano. Figura única, hecha por completo a mano.$fx$
 WHERE lang = 'es' AND short_text = $fx$Pequeña muñeca con un hatillo de leña. Hecha en arcilla polimérica, pintada con acrílicos y vestida a mano. Figura única, hecha por completo a mano.$fx$
   AND figurine_id = (SELECT id FROM figurines WHERE slug = 'small-doll-gnome-with-fancy-bag');

-- small-doll-gnome-with-fancy-bag (es): full_description
UPDATE figurine_translations SET full_description = $fx$Te presentamos a este pequeño y curioso caminante del bosque, vestido con ropa rústica hecha a mano y cargado con una gran mochila de colores llena de leña. Con su alto sombrero torcido, su rostro expresivo, el pelo algo revuelto y los largos zapatos puntiagudos, parece llevar ya bastante tiempo viajando por el bosque y, seguramente, tiene alguna que otra historia que contar.

Su enorme mochila está hecha con telas de textura marcada y decorada con detalles tejidos de colores, que contrastan de maravilla con su sencilla ropa oscura. Lleva un bastón de madera natural, que le da el aire de un viajero de los de antes, siempre listo para seguir su camino por otro sendero del bosque.

La muñeca está modelada y pintada a mano por completo, y cada detalle se ha hecho por separado. Un armazón de alambre en su interior permite mover ligeramente brazos y piernas para cambiar la postura. La ropa rústica, el alto sombrero hecho a mano, la mochila texturada, los botoncitos, los zapatos puntiagudos y el bastón de madera natural le dan su carácter único de viejo mundo.

Con unos 10-11 cm de altura, quedaría muy bien en una colección de fantasía o de personajes del bosque, en una vitrina de miniaturas, en un diorama, en una escena de casa de muñecas o, sencillamente, en un lugar especial de una estantería.

Es una muñeca artística verdaderamente única, hecha a mano: nunca habrá otra exactamente igual.$fx$
 WHERE lang = 'es' AND full_description = $fx$Te presentamos a este pequeño y curioso caminante del bosque, vestido con ropa rústica hecha a mano y cargado con una gran mochila de colores llena de tesoros misteriosos. Con su alto sombrero torcido, su rostro expresivo, el pelo algo revuelto y los largos zapatos puntiagudos, parece llevar ya bastante tiempo viajando por el bosque y, seguramente, tiene alguna que otra historia que contar.

Su enorme mochila está hecha con telas de textura marcada y decorada con detalles tejidos de colores, que contrastan de maravilla con su sencilla ropa oscura. Lleva un bastón de madera natural, que le da el aire de un viajero de los de antes, siempre listo para seguir su camino por otro sendero del bosque.

La muñeca está modelada y pintada a mano por completo, y cada detalle se ha hecho por separado. Un armazón de alambre en su interior permite mover ligeramente brazos y piernas para cambiar la postura. La ropa rústica, el alto sombrero hecho a mano, la mochila texturada, los botoncitos, los zapatos puntiagudos y el bastón de madera natural le dan su carácter único de viejo mundo.

Con unos 10-11 cm de altura, quedaría muy bien en una colección de fantasía o de personajes del bosque, en una vitrina de miniaturas, en un diorama, en una escena de casa de muñecas o, sencillamente, en un lugar especial de una estantería.

Es una muñeca artística verdaderamente única, hecha a mano: nunca habrá otra exactamente igual.$fx$
   AND figurine_id = (SELECT id FROM figurines WHERE slug = 'small-doll-gnome-with-fancy-bag');

-- small-doll-gnome-with-fancy-bag (it): short_text
UPDATE figurine_translations SET short_text = $fx$Piccola bambola: gnomo con uno zaino colorato pieno di sterpi. In argilla polimerica, dipinta con colori acrilici e vestita a mano. Figura interamente fatta a mano, pezzo unico.$fx$
 WHERE lang = 'it' AND short_text = $fx$Piccola bambola con una sacca di sterpi. In argilla polimerica, dipinta con colori acrilici e vestita a mano. Figura interamente fatta a mano, pezzo unico.$fx$
   AND figurine_id = (SELECT id FROM figurines WHERE slug = 'small-doll-gnome-with-fancy-bag');

-- small-doll-gnome-with-fancy-bag (it): full_description
UPDATE figurine_translations SET full_description = $fx$Ecco un piccolo viandante dei boschi, curioso, vestito di abiti rustici cuciti a mano e con un grande zaino colorato pieno di sterpi. Con l’alto cappello storto, il viso espressivo, i capelli un po’ spettinati e le lunghe scarpe a punta, sembra in cammino nel bosco da parecchio tempo, e probabilmente ha una storia o due da raccontare.

Lo zaino, sproporzionato, è fatto di tessuti ricchi di trama e decorato con dettagli intrecciati colorati, in bel contrasto con i suoi abiti semplici e scuri. Porta un bastone da cammino di legno naturale, che gli dà l’aspetto di un viaggiatore d’altri tempi, sempre pronto a riprendere la strada lungo un altro sentiero del bosco.

La bambola è interamente modellata e dipinta a mano, e ogni dettaglio è realizzato uno per uno. Un’armatura di filo metallico all’interno permette di muovere leggermente braccia e gambe per metterlo in posa. Gli abiti rustici, l’alto cappello fatto a mano, lo zaino materico, i minuscoli bottoni, le scarpe a punta e il bastone di legno naturale contribuiscono al suo carattere d’altri tempi.

Alto circa 10–11 cm, trova posto in una raccolta fantasy o di creature del bosco, in una vetrina di miniature, in un diorama, in una scena di casa delle bambole o semplicemente in un angolo speciale di uno scaffale.

È una vera bambola d’artista fatta a mano in un unico esemplare: non ce ne sarà mai un’altra identica.$fx$
 WHERE lang = 'it' AND full_description = $fx$Ecco un piccolo viandante dei boschi, curioso, vestito di abiti rustici cuciti a mano e con un grande zaino colorato pieno di tesori misteriosi. Con l’alto cappello storto, il viso espressivo, i capelli un po’ spettinati e le lunghe scarpe a punta, sembra in cammino nel bosco da parecchio tempo, e probabilmente ha una storia o due da raccontare.

Lo zaino, sproporzionato, è fatto di tessuti ricchi di trama e decorato con dettagli intrecciati colorati, in bel contrasto con i suoi abiti semplici e scuri. Porta un bastone da cammino di legno naturale, che gli dà l’aspetto di un viaggiatore d’altri tempi, sempre pronto a riprendere la strada lungo un altro sentiero del bosco.

La bambola è interamente modellata e dipinta a mano, e ogni dettaglio è realizzato uno per uno. Un’armatura di filo metallico all’interno permette di muovere leggermente braccia e gambe per metterlo in posa. Gli abiti rustici, l’alto cappello fatto a mano, lo zaino materico, i minuscoli bottoni, le scarpe a punta e il bastone di legno naturale contribuiscono al suo carattere d’altri tempi.

Alto circa 10–11 cm, trova posto in una raccolta fantasy o di creature del bosco, in una vetrina di miniature, in un diorama, in una scena di casa delle bambole o semplicemente in un angolo speciale di uno scaffale.

È una vera bambola d’artista fatta a mano in un unico esemplare: non ce ne sarà mai un’altra identica.$fx$
   AND figurine_id = (SELECT id FROM figurines WHERE slug = 'small-doll-gnome-with-fancy-bag');
