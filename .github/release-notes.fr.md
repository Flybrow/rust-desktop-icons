## Nouveautés de Rust Desktop Icons 1.2.0

### Nouveau

- **Choisissez comment chaque onglet affiche ses éléments, comme dans l'Explorateur :** faites un clic droit sur une zone, ouvrez le nouveau menu Affichage et choisissez Icônes, Liste, Détails, Mosaïques ou Contenu. Le choix ne concerne que l'onglet en cours : chaque onglet d'une zone peut avoir son propre affichage. Ce menu remplace « Afficher en liste » du menu Apparence ; Paramètres → Icônes → « Affichage en liste par défaut » règle toujours l'affichage des onglets qui n'en ont pas.
- **Affichage Détails avec des colonnes :** chaque élément occupe une ligne avec son nom, sa date de modification, son type et sa taille, comme dans l'Explorateur. Cliquez sur le titre d'une colonne pour trier les éléments selon cette colonne ; cliquez de nouveau pour inverser l'ordre.
- **Affichage Mosaïques :** une grande icône avec le nom, le type et la taille de l'élément écrits à côté.
- **Affichage Contenu :** une ligne large par élément, avec le nom et le type à gauche, la date de modification et la taille à droite.
- **Tri par date de création :** un nouveau choix dans le menu de la zone → Trier par, à côté du nom, du type, de la date de modification et de la taille.

### Améliorations

- **Enroulement et déroulement plus fluides :** l'animation d'une zone enroulée suit maintenant la fréquence de votre écran, avec des images régulières. Le petit à-coup au début de chaque déroulement a disparu.
- **La loupe d'un portail est maintenant au bout de son propre onglet :** dans une zone à plusieurs onglets, elle était tout au bout de la barre de titre, sur un autre onglet, alors qu'elle filtre l'onglet actif.

### Corrections

- **Une zone enroulée reste ouverte quand vous y ouvrez un dossier plus petit :** la zone rétrécissait, la souris se retrouvait en dehors, puis la zone s'enroulait et revenait à ses propres éléments.
- **Plus de fermeture avec une zone latérale très étroite :** une zone avec la barre de titre sur le côté, affichée en liste et rendue très étroite, pouvait fermer l'application.
- **Un fichier de configuration illisible n'est plus jamais écrasé :** s'il est verrouillé par un autre programme au démarrage, l'application le signale et n'enregistre plus rien jusqu'au redémarrage, au lieu de remplacer votre disposition.
- **« Démarrer avec Windows » et les messages de mise à jour disent vrai :** l'option ne reste cochée que si Windows l'a acceptée, et un téléchargement de mise à jour qui échoue le dit, au lieu de « Impossible de vérifier les mises à jour ».
- **Vider « Filtrer les fichiers… » réaffiche maintenant tous les fichiers :** vider le champ puis appuyer sur Entrée gardait l'ancien filtre. Le champ ne commence plus non plus par `*.pdf` : l'ouvrir puis cliquer ailleurs ne pose plus de filtre par erreur.
