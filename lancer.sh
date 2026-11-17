. COMPILER le compilateur depuis ses sources

Le binaire livré marche déjà, mais compiler toi-même le cœur Rust (zéro dépendance, ça marche hors ligne) :
bash

cargo build --release
cp target/release/rustava ./rustava      # remplace le binaire livré par LE TIEN

    Si cargo manque : sudo apt install cargo rustc (Debian/Ubuntu), ou rustup.rs.

Puis prouve que TON build tourne :
bash

bash tests/regression.sh                 # doit finir : TOUS LES TESTS PASSENT

2. INSTALLER (pour lancer rustava depuis n'importe où)
bash

bash installe.sh

Il vérifie gcc, pose le compilateur dans ~/.local/bin et le runtime dans ~/.local/share/rustava/, puis fait un test de santé. Ensuite :
bash

rustava run exemples/spec/spec-01-bonjour.rv     # « Bonjour, je suis Rustava »

3. VOIR L'INTÉRIEUR DU CŒUR
bash

# le cœur C — l'arène mémoire, rv_allouer, afficher, les threads légers :
less src/runtime.h

# la chaîne de compilation, étape par étape :
less src/lexer.rs        # étape 1 : le lexer
less src/parser.rs       # étape 2 : le parser (match, try/catch, spawn...)
less src/checker.rs      # étape 3 : le type-checker (let immuable, contrats)
less src/codegen.rs      # étape 4 : la génération du C

# LE plus fort : le compilateur te montre le C qu'il produit :
./rustava c exemples/spec/spec-01-bonjour.rv | less

# vérifier sans exécuter / produire le binaire natif :
./rustava check exemples/spec/spec-05-classes.rv
./rustava build exemples/spec/spec-04-fonctions.rv   # binaire natif à côté du .rv

# la taille du cœur, fichier par fichier :
wc -l src/*.rs src/runtime.h
