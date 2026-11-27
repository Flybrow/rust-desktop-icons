## Nouveautés de Rust Desktop Icons 1.3.0

### Nouveau

- **Thème Glass :** les zones peuvent devenir des plaques de verre dépoli, avec le fond d'écran légèrement flouté derrière elles. Pas de cadre, une barre de titre légère et l'onglet actif dans une pastille discrète. Activez-le dans Paramètres → Apparence par défaut → Thème → Glass, ou pour une seule zone : clic droit → Apparence → Thème. Le verre est sombre ou clair selon la couleur de la zone, et se fonce ou s'éclaircit juste ce qu'il faut sur un fond très clair ou très sombre pour que les noms restent lisibles. Tous vos réglages d'apparence s'appliquent toujours : couleur, opacité (la force de la teinte de la zone sur le verre, du fond d'écran seul à une couleur presque unie), couleur de la barre de titre, coins, taille des icônes et des noms, alignement et taille du titre.
- **Mode remplace l'ancien réglage Thème :** choisissez sombre, clair ou automatique (comme Windows) à part, maintenant que Thème choisit Défaut ou Glass. Il est en haut de Paramètres → Apparence par défaut, juste sous Thème.
- **Coins plus arrondis :** les zones peuvent maintenant être arrondies de 12 ou 16 px. Choisissez-les dans Paramètres → Apparence par défaut → Coins ou dans le menu Apparence de chaque zone.
- **Importer une sauvegarde faite sur un autre PC :** la disposition et les réglages reviennent sans mélanger les fichiers de cet autre PC avec les vôtres. Les fichiers déjà dans vos zones restent où ils sont ; ceux qui n'ont plus de zone reviennent dans une zone « Fichiers récupérés ».

### Améliorations

- **Une zone repliée prend la hauteur de chaque onglet ouvert :** passer à un onglet qui contient moins d'éléments réduit maintenant la zone ouverte à sa taille, et chaque zone prend en douceur la hauteur de l'onglet choisi au lieu de sauter. Elle garde toujours sa taille quand vous ouvrez un dossier plus petit à l'intérieur.
- **Zones repliées plus fluides :** une zone se déroule toujours visiblement, même quand elle doit d'abord être redessinée, et se replie dès que la souris la quitte.
- **Les zones restent sur le bureau avec Win+D sous Windows 10 :** « Afficher le bureau » (Win+D ou le coin de la barre des tâches) ne masque plus les zones, y compris juste après l'ouverture de session (ticket n° 4).
- **Plus rapide avec les grandes zones :** trier un onglet dans votre propre ordre, chercher dans toutes les zones et « Garder les fichiers sur le bureau » avec des milliers de fichiers sont bien plus rapides.
- **Moins de mémoire pour les miniatures :** une zone ne garde que les aperçus de son onglet ouvert, dans une limite fixe ; au-delà, les éléments montrent leur icône habituelle.
- **Mises à jour plus sûres :** les téléchargements sont vérifiés plus strictement et uniquement en HTTPS, les connexions lentes n'échouent plus, et la mise à jour fonctionne quand le nom d'utilisateur Windows contient un « ! ».

### Corrections

- **Alt+F4 ne ferme plus une zone.**
- **Les zones « couleur du fond d'écran » gardent la même couleur sur tous les onglets.**
- **Suppr pendant la saisie dans le filtre d'une zone ne supprime plus les fichiers sélectionnés.**
- **Renommer un onglet renomme toujours celui sur lequel vous avez commencé,** même si la souris passe sur un autre onglet entre-temps.
- **Votre propre ordre est conservé :** réordonner les icônes pendant un filtre, ou choisir un tri pendant qu'un dossier est ouvert dans la zone, ne retire plus les éléments masqués de votre ordre.
- **Annuler remet un fichier sur le bureau pour de bon :** le tri automatique ne le renvoie plus aussitôt dans une zone.
- **Déplacer des fichiers n'écrase jamais un fichier :** si un fichier du même nom apparaît entre-temps, le nom libre suivant est utilisé. Entre deux disques, l'original n'est supprimé qu'une fois sa copie complète et inchangée.
- **Désinstaller ne supprime jamais vos fichiers :** si le dossier d'une zone ne peut pas être lu, rien n'est supprimé et vous êtes prévenu.
- **Garder les fichiers sur le bureau :** un fichier qui disparaît un instant (enregistré par Office, bureau réseau qui décroche) reste dans sa zone, et une icône sortie d'une zone atterrit sous la souris.
- **Une seule valeur erronée ne réinitialise plus votre disposition :** un fichier de réglages abîmé garde désormais toutes les zones qu'il peut lire.
- **Les icônes du bureau masquées dans l'Explorateur restent masquées** après un masquage rapide ou un redémarrage de l'application.
- **Déposer un lien web ou du texte sur une zone** affiche maintenant le curseur « interdit » au lieu de ne rien faire.
- **Paramètres :** un double-clic sur une ligne n'agit plus deux fois.
- **Mode confidentialité :** « Quand le bureau est utilisé » compte aussi un clic sur une zone.
- **Les nouvelles zones restent dans l'écran,** même créées près d'un bord.
- **Les entrées du clic droit sur le bureau fonctionnent quand l'application tourne en administrateur,** de même que l'icône de la zone de notification et son retour après un redémarrage de l'Explorateur.
- **De nombreuses petites corrections** issues d'une relecture complète du code : plantages rares, fichiers affichés dans le mauvais onglet après un changement rapide de dossier, alertes répétées toutes les heures pour un fichier verrouillé, bureaux virtuels plus suivis après un redémarrage de l'Explorateur.
