## Nouveautés de Rust Desktop Icons 1.1.0

### Nouveau

- **Icônes masquées directement dans l'Explorateur :** avec « Garder les fichiers sur le Bureau », les icônes rangées dans une zone sont maintenant masquées par l'Explorateur lui-même, comme le fait Fences. Elles ne clignotent plus après F5, un renommage ou un nouveau fichier. Si cela ne peut pas fonctionner sur votre PC, l'ancienne méthode (une case cachée dans un coin) est utilisée.
- **Filtrer une zone en tapant :** Ctrl+F dans une zone, un clic sur la loupe d'un portail ou le menu de la zone. Un champ de recherche apparaît dans la barre de titre et seules les icônes correspondantes restent (accents ignorés). Entrée ouvre la première, Échap ferme le filtre.
- **Mode confidentialité :** pour un partage d'écran ou des personnes à côté, les noms deviennent des barres grises, les icônes des carrés unis et, si vous le voulez, les titres des points. Choisissez quand ils reviennent : souris sur la zone, bureau utilisé, ou jamais. Paramètres → Confidentialité, ou le menu de l'icône de notification.
- **Affichage en liste :** « Afficher en liste » met un élément par ligne, une petite icône avec son nom à côté. Pour les zones comme pour les portails.
- **Thème sombre, clair ou automatique :** pour les zones, les menus, les paramètres et les fenêtres. Automatique suit Windows. Les couleurs choisies vous-même sont conservées.
- **Coins arrondis :** carrés, 4, 6 ou 8 px, pour toutes les zones ou une seule.
- **Zones portail en une étape :** « Nouvelle zone portail… » dans le menu de l'icône de notification et « Créer une zone portail RDI… » dans le clic droit du bureau créent une zone qui affiche un dossier.
- **Revenir au début en cliquant ailleurs :** un dossier ouvert dans une zone peut se refermer tout seul quand vous cliquez en dehors de la zone.
- **Tout masquer peut laisser les autres icônes du bureau visibles :** nouvelle option « Tout masquer : aussi les icônes du bureau » dans Paramètres → Bureau.

### Corrections

- **Icônes nettes à toutes les tailles :** les icônes redimensionnées avec Ctrl+molette ou le réglage de taille ne sont plus floues ni crénelées entre les tailles fournies par Windows.
- **Titres chinois, japonais et coréens dans une barre latérale :** ils sont empilés droits, de haut en bas, au lieu d'être tournés sur le côté. Merci à @asir66 pour cette contribution.
- **Une zone enroulée passée en titre vertical** s'affiche correctement tout de suite, pas seulement après avoir été déroulée.
- **Les zones enroulées gardent leurs coins arrondis** des quatre côtés.
- **Plus rapide après un redémarrage de l'Explorateur :** les icônes masquées n'apparaissent plus qu'une fraction de seconde au lieu de jusqu'à 3 secondes.
