Chapitre 6. Fonctions
Une fonction se déclare avec fn, un nom, des paramètres typés et, après une flèche ->, le type de retour si
la fonction rend une valeur. Sans flèche, la fonction ne rend rien. L'instruction return quitte la fonction ; il
peut être suivi d'une valeur ou utilisé seul. Les paramètres exigent tous une annotation de type — le
compilateur n'infère pas les signatures, il vérifie tout le reste.
Chaque fichier exécutable doit fournir le point d'entrée « fn main() » au niveau du fichier : sans
paramètre, sans type de retour. Ces deux contraintes sont vérifiées par le type-checker avec des messages
explicites. Les fonctions sont récursives sans déclaration préalable — l'ordre de déclaration dans le
fichier n'importe pas.
spec-04-fonctions.rv — récursion prouvée
fn fact(n: int) -> int {
if n <= 1 {
return 1;
}
return n * fact(n - 1);
}
fn saluer(prenom: string, fois: int) {
for i in 0..fois {
afficher(prenom);
}
}
fn main() {
afficher(fact(5)); // 120
saluer("Rustava", 2);
}
Le passage d'arguments emprunte automatiquement les valeurs : une chaîne ou un objet passe par
référence sans syntaxe spéciale, et le compilateur se charge des détails (chapitre 11). C'est le sens du
commentaire « passage par référence automatique » qui traverse les exemples du langage depuis sa
première heure.
Chapitre 7. Classes et interfaces
Une classe porte des champs et des méthodes. Les champs se déclarent avec let ou var suivi d'un type —
et sans initialisation : c'est le constructeur qui les initialise, et le constructeur est la méthode portant le
nom de la classe. Dans les méthodes, le récepteur s'appelle self ; this est un alias exact de self, offert pour
que les mains venues de Java ne trébuchent pas. L'instanciation s'écrit new Point(2, 3) — les parenthèses
sont optionnelles quand il n'y a pas d'arguments.
spec-05-classes.rv — champs, constructeur, méthodes
RUSTAVA 1.0 — Spécification du langage
Aissa 6
class Point {
let x: int;
let y: int;
fn Point(px: int, py: int) {
self.x = px;
self.y = py;
}
fn somme() -> int {
return self.x + self.y;
}
fn deplacer(dx: int, dy: int) {
self.x += dx;
self.y += dy;
}
}
fn main() {
let p = new Point(2, 3);
afficher(p.somme()); // 5
p.deplacer(10, 20);
afficher(p.x); // 12
}
7.1 Les interfaces : le contrat sans l'héritage
Une interface déclare des signatures de méthodes terminées par un point-virgule. Une classe qui
implémente une interface doit fournir toutes les méthodes du contrat, sinon le compilateur refuse avec le
message « contrat incomplet ». Une classe peut implémenter plusieurs interfaces, séparées par des
virgules — c'est la composition par contrats, voie choisie à la place de l'héritage : extends est refusé
partout, y compris entre interfaces.
7.2 Le dispatch dynamique
Le polymorphisme passe par les signatures : un paramètre typé par une interface accepte n'importe quel
objet d'une classe qui l'implémente, et c'est l'objet concret qui décide, à l'exécution, de la méthode
appelée. Le référentiel des méthodes vit dans chaque objet — aucune table virtuelle à déclarer, aucune
annotation.
spec-06-interfaces.rv — deux interfaces, dispatch réel
RUSTAVA 1.0 — Spécification du langage
Aissa 7
interface Volant {
fn decrire() -> string;
}
interface Nommable {
fn nom() -> string;
}
class Oiseau implements Nommable, Volant {
fn nom() -> string { return "oiseau"; }
fn decrire() -> string { return "je vole"; }
}
class Avion implements Nommable, Volant {
fn nom() -> string { return "avion"; }
fn decrire() -> string { return "je vole avec des ailes"; }
}
fn presenter(v: Volant) {
afficher(v.decrire()); // l'objet concret décide
}
fn main() {
presenter(new Oiseau()); // je vole
presenter(new Avion()); // je vole avec des ailes
}
Chapitre 8. Erreurs : try, catch, throw
La gestion d'erreur est celle de Java, assumée : un bloc try entoure le code risqué, catch récupère l'erreur
active et throw en lance une. La variable de catch est optionnelle : « catch (e) { } » nomme l'erreur, «
catch { } » l'attrape sans la nommer. Le message d'une erreur est une chaîne : throw exige une string, et le
type-checker refuse de lancer autre chose.
Certaines erreurs d'exécution suivent le même canal : la division par zéro, par exemple, ne plante pas le
programme mais lance une erreur attrapable par try/catch. Une erreur non attrapée remonte jusqu'au
point d'entrée et est signalée proprement à la fin de l'exécution, avec son message.
spec-07-erreurs.rv — les trois formes prouvées
RUSTAVA 1.0 — Spécification du langage
Aissa 8
fn diviser(a: int, b: int) -> int {
return a / b; // division par zéro : attrapable
}
fn main() {
try {
throw "problème volontaire";
afficher("jamais affiché");
} catch (e) {
afficher("attrapé :");
afficher(e);
}
try {
afficher(diviser(10, 0));
} catch {
afficher("division par zéro interceptée");
}
try {
afficher(diviser(10, 2));
} catch {
afficher("jamais affiché");
}
}
Sortie réelle de ce programme : « attrapé : » puis « problème volontaire », puis « division par zéro
interceptée », puis « 5 » — le troisième bloc réussit et le catch reste silencieux. Le flux d'exécution du try
s'interrompt au throw : les instructions qui le suivent, dans le même bloc, ne s'exécutent pas.
Chapitre 9. Concurrence : spawn
La concurrence du mariage tient en un mot-clé : spawn. Écrit devant un appel de fonction ou de méthode
défini par l'utilisateur, il lance l'appel dans un thread léger détaché et ne l'attend pas. Le programme
continue sur le fil principal pendant que les fils lancés courent en même temps — l'ordre d'affichage
entre plusieurs spawn n'est pas garanti, et c'est la preuve visible de la vraie concurrence.
Deux précisions font partie du contrat. D'abord spawn s'applique aux fonctions du programme : les
fonctions intégrées comme afficher ne sont pas des cibles valides, on les enveloppe dans une fonction à
soi. Ensuite, un thread détaché peut être coupé si le programme principal se termine immédiatement :
quand on veut voir les affichages des threads, on termine le main par dormir_ms(100) pour leur laisser le
temps de finir. Ce comportement est voulu — spawn ne crée pas de dépendance de synchronisation.
spec-08-concurrence.rv — trois threads lancés
RUSTAVA 1.0 — Spécification du langage
Aissa 9
fn travailler(id: int) {
afficher("thread");
afficher(id);
}
fn main() {
spawn travailler(1);
spawn travailler(2);
spawn travailler(3);
dormir_ms(100); // laisse les threads finir
afficher("fin");
}
Chapitre 10. Bibliothèque intégrée
Le runtime C embarque avec chaque binaire fournit l'ensemble des fonctions natives du langage. Il est
volontairement petit : tout le reste s'écrit en RUSTAVA. La table ci-dessous fait foi — noms exacts,
arguments et effet.
Fonction Arguments Effet
afficher(v) / print(v) une valeur de tout type affiche la valeur et un retour à la ligne
longueur(x) une string ou un tableau rend la longueur (int)
x.longueur() / x.taille() méthode de string ou tableau même résultat, en forme de méthode
caractere(s, i) une string, un index rend le i-ème caractère (string)
horloge_ms() aucun temps écoulé en millisecondes (int)
dormir_ms(n) un int met le fil courant en pause n millisecondes
Table 10.1 — les fonctions intégrées
afficher et print sont deux noms pour la même fonction — le nom court de Java et le nom long du
mariage. Un objet s'affiche entre crochets angulaires avec son nom de classe (par exemple ). Les
booléens s'affichent « vrai » et « faux », les chaînes sans guillemets.
spec-09-tableaux.rv — longueurs et indexations
RUSTAVA 1.0 — Spécification du langage
Aissa 10
fn main() {
let t = [10, 20, 30];
afficher(t[0]); // 10
afficher(longueur(t)); // 3 : fonction libre
afficher(t.longueur()); // 3 : méthode
afficher(t.taille()); // 3 : alias de méthode
var u = [1, 2, 3];
u[2] = 99; // écriture par index
u[0] += 41; // composée par index
afficher(u[2]); // 99
afficher(u[0]); // 42
let mot = "rustava";
afficher(longueur(mot)); // 7
afficher(caractere(mot, 0)); // r
}
Chapitre 11. Le modèle mémoire : l'arène déterministe
RUSTAVA ne pose pas le dilemme classique entre le ramasse-miettes de Java et la discipline manuelle de
Rust. Toutes les allocations — chaînes, tableaux, objets — vivent dans une arène : un bloc de gestion
unique dans lequel le programme écrit pendant toute son exécution, sans jamais libérer au coup par coup.
À la fin du programme, l'arène entière est rendue d'un coup, par un point d'accroche du système (atexit).
Il n'y a donc ni pauses de collecte, ni fuites invisibles, ni compteur de références : le coût mémoire est
exactement celui des allocations, et la sortie est déterministe.
Le borrow checker du mariage est simplifié au point d'être invisible. Le type-checker suit qui porte quelle
valeur et refuse les mélanges dangereux (int/float, tableaux hétérogènes) avant l'exécution ; le passage
d'arguments emprunte automatiquement — le mot-clé borrow existe pour le dire explicitement, mais
aucun programme n'en a besoin pour compiler. Cette décision fait partie du contrat du langage : le
programmeur pense son algorithme, le compilateur pense la mémoire.
Conséquence pratique pour l'outillage : le runtime C vit dans un fichier unique, src/runtime.h, qu'on peut
lire comme un manuel — chaque fonction y est petite, commentée et sans magie. La chaîne d'exécution
complète est : le compilateur produit du C, gcc produit le binaire, le binaire embarque le runtime par
inclusion.
RUSTAVA 1.0 — Spécification du langage
Aissa 11
Chapitre 12. Le compilateur et son outillage
Le compilateur rustava est écrit en Rust pur, sans dépendance externe, en quatre étapes nettes. Chaque
étape a son fichier, ses erreurs et ses tests ; une erreur en amont arrête la chaîne avec un message en
français, ligne et colonne.
Étape Fichier Rôle
1. Lexer lexer.rs découpe le code en tokens ; erreurs de lexique en français
2. Parser parser.rs construit l'arbre syntaxique (classes, match, try/catch, spawn)
3. Type checker checker.rs types, immuabilité, contrats d'interfaces ; le borrow simplifié
4. Génération C codegen.rs traduit l'arbre en C, avec le runtime maison runtime.h
Table 12.1 — les quatre étapes
L'outillage autour du compilateur tient en quatre commandes. run compile et exécute ; build produit le
binaire natif à côté du fichier source ; check vérifie sans rien produire ; c affiche le C généré — la porte
ouverte pour comprendre ou auditer la traduction. La suite de non-régression, tests/regression.sh, rejoue
les exemples officiels et les refus du langage : elle doit toujours passer en entier avant toute livraison.
les quatre commandes
./rustava run exemples/spec/spec-01-bonjour.rv # exécute
./rustava build exemples/spec/spec-01-bonjour.rv # binaire natif
./rustava check exemples/spec/spec-01-bonjour.rv # vérifie
./rustava c exemples/spec/spec-01-bonjour.rv # montre le C
Chapitre 13. Les refus du compilateur
Un langage se juge à ce qu'il refuse. Chaque refus ci-dessous est un cas de test réel de la suite de
non-régression : le code fautif à gauche, le message réel du compilateur à droite — en français, avec la
ligne. Ces messages font partie du langage au même titre que les mots-clés : ils sont stables, testés et
attendus.
Code fautif Message du compilateur
class B extends A { } Rustava refuse l'héritage (« extends ») : compose avec des interfaces
(implements) — c'est le mariage choisi
let x = 10; x = 20; Erreur de type : la variable « x » est immuable (let) : réaffectation interdite
class Chien implements Animal —
crier manquant
Erreur de type : la classe « Chien » implémente « Animal » mais le contrat
est incomplet
RUSTAVA 1.0 — Spécification du langage
Aissa 12
Code fautif Message du compilateur
if x = 5 { } Erreur de syntaxe : j'ai trouvé « = », j'attendais « { » (comparer s'écrit ==)
throw 42; Erreur de type : throw attend une string (le message d'erreur) : attendu «
string », trouvé « int »
fn f(a: int, b: int,) — virgule finale Erreur de syntaxe : j'attendais un nom de paramètre ; j'ai trouvé « ) »
class chien { } une classe devrait commencer par une majuscule (« chien »)
let x = 1 + 1.5; Erreur de type : mélange int/float refusé
Table 13.1 — les refus qui font le contrat
La règle commune à tous ces messages : dire ce qui manque, où, et comment réparer. Le compilateur ne
juge pas le style, il fait respecter le contrat du mariage — et quand il refuse, il explique.
Chapitre 14. Grammaire EBNF complète
La grammaire ci-dessous décrit l'intégralité du langage au niveau du parser, dans la notation EBNF. Elle
est la même que celle livrée dans le fichier grammaire-rustava.ebnf à la racine du dépôt — les deux textes
sont identiques et toute évolution du langage passe par les deux en même temps. Les terminaux entre
guillemets sont les mots-clés et symboles exacts du chapitre 2.
grammaire-rustava.ebnf — texte officiel intégral
--- grammaire-rustava.ebnf (version 1.0) ---
programme = { declaration } ;
declaration = classe | interface_decl | fonction ;
classe = "class" MAJ_IDENT [ "implements" MAJ_IDENT { "," MAJ_IDENT } ]
"{" { champ | methode } "}" ;
champ = ( "let" | "var" ) IDENT ":" type ";" ;
methode = "fn" IDENT "(" [ params ] ")" [ "->" type ] bloc ;
interface_decl = "interface" MAJ_IDENT "{" { signature } "}" ;
signature = "fn" IDENT "(" [ params ] ")" [ "->" type ] ";" ;
fonction = "fn" IDENT "(" [ params ] ")" [ "->" type ] bloc ;
params = param { "," param } ;
param = IDENT ":" type ;
type = "int" | "float" | "bool" | "string" | MAJ_IDENT ;
bloc = "{" { instruction } "}" ;
instruction = declaration_var | si | tant | pour | match | essai
| jeter | retour | envoi | affectation | expression ";"
| bloc | ";" ;
declaration_var = ( "let" | "var" ) IDENT [ ":" type ] "=" expression ";" ;
si = "if" condition bloc { "else" "if" condition bloc }
[ "else" bloc ] ;
RUSTAVA 1.0 — Spécification du langage
Aissa 13
grammaire-rustava.ebnf — texte officiel intégral — suite 1
tant = "while" condition bloc ;
pour = "for" IDENT "in" ( intervalle | expression ) bloc ;
intervalle = expression ".." expression ;
match = "match" condition "{" { bras } "}" ;
bras = motif "=>" ( expression [ ";" | "," ] | bloc ) ;
motif = int_lit | float_lit | chaine_lit | "true" | "false" | "_" ;
essai = "try" bloc "catch" [ "(" IDENT ")" ] bloc ;
jeter = "throw" expression ";" ;
retour = "return" [ expression ] ";" ;
envoi = "spawn" appel ";" ;
affectation = cible ( "=" | "+=" | "-=" | "*=" | "/=" ) expression ";" ;
cible = IDENT | postfixe "." IDENT | postfixe "[" expression "]" ;
expression = ou_logique ;
ou_logique = et_logique { "||" et_logique } ;
et_logique = egalite { "&&" egalite } ;
egalite = comparaison { ( "==" | "!=" ) comparaison } ;
comparaison = additif { ( "<" | "<=" | ">" | ">=" ) additif } ;
additif = multiplicatif { ( "+" | "-" ) multiplicatif } ;
multiplicatif = unaire { ( "*" | "/" | "%" ) unaire } ;
unaire = ( "-" | "!" | "borrow" ) unaire | postfixe ;
postfixe = primaire { "." IDENT [ "(" [ args ] ")" ]
| "[" expression "]" } ;
primaire = int_lit | float_lit | chaine_lit | "true" | "false"
| "self" | "this" | "(" expression ")"
grammaire-rustava.ebnf — texte officiel intégral — suite 2
| "new" MAJ_IDENT [ "(" [ args ] ")" ]
| "[" [ expression { "," expression } ] "]"
| IDENT [ "(" [ args ] ")" ] ;
args = expression { "," expression } ;
MAJ_IDENT = lettre_majuscule { lettre | chiffre | "_" } ;
IDENT = ( lettre | "_" ) { lettre | chiffre | "_" } ;
int_lit = chiffre { chiffre | "_" } ;
float_lit = int_lit "." chiffre { chiffre } ;
chaine_lit = '"' { caractère | échappement } '"' ;
échappement = "\" ( "n" | "t" | "\" | '"' | "0" ) ;
RUSTAVA 1.0 — Spécification du langage
Aissa 14
Chapitre 15. Le programme complet de démonstration
Le fichier demo-complete.rv, livré dans le dossier exemples du dépôt, traverse tous les chapitres de cette
spécification en un seul programme : commentaires imbriqués, variables, opérateurs logiques,
échappements, arithmétique mixte, interfaces et dispatch dynamique, concurrence, erreurs. Il compile et
s'exécute sans erreur — sa sortie réelle accompagne le listing.
demo-complete.rv — le programme de démonstration
fn main() {
let createur = "Aissa";
var annee = 2025;
annee += 1;
if annee >= 2026 && !(annee > 2100) {
afficher("version");
afficher(annee);
}
afficher("ligne un\nligne deux\tfin");
afficher(7 / 2);
afficher(7.0 / 2.0);
afficher(7 % 3);
animal(new Chien());
animal(new Chat());
spawn coucou();
dormir_ms(100);
try {
throw "fin annoncée";
} catch (e) {
afficher(e);
}
}
fn coucou() { afficher("thread lancé"); }
interface Animal { fn crier() -> string; }
class Chien implements Animal {
fn crier() -> string { return "ouaf"; }
demo-complete.rv — le programme de démonstration — suite 1
}
class Chat implements Animal {
fn crier() -> string { return "miaou"; }
}
fn animal(a: Animal) { afficher(a.crier()); }
Sortie réelle, dans l'ordre du fil principal : « version », « 2026 », « ligne un », « ligne deux » suivie d'une
tabulation puis « fin », « 3 », « 3.5 », « 1 », puis le dispatch « ouaf » puis « miaou », puis l'affichage du
thread lancé, puis « fin annoncée » attrapé par le catch. L'ordre exact des affichages du thread dépend de
l'ordonnanceur — c'est la vraie concurrence du chapitre 9.
RUSTAVA 1.0 — Spécification du langage
Aissa
