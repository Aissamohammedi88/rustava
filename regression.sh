#!/usr/bin/env bash
# RUSTAVA — suite de non-régression (exécutée avant chaque livraison)
set -u
DIR=$(cd "$(dirname "$0")/.." && pwd)
RV="$DIR/target/release/rustava"
ECHO=OK ; ECHEC=0

attend() { # attend "fichier" "sortie attendue en 1 ligne" "description"
    local f="$1" ; shift
    local attendu="$1" ; shift
    local desc="$1"
    local sortie
    sortie=$("$RV" run "$f" 2>&1)
    local code=$?
    if [[ $code -ne 0 ]] ; then
        echo "[KO] $desc (code $code)"
        ECHEC=$((ECHEC+1))
        return
    fi
    if [[ "$sortie" != *"$attendu"* ]] ; then
        echo "[KO] $desc (attendu « $attendu », reçu : ${sortie//$'\n'/ ; })"
        ECHEC=$((ECHEC+1))
        return
    fi
    echo "[OK] $desc"
}

refuse() { # refuse "fichier" "fragment du message" "description"
    local f="$1" ; shift
    local fragment="$1" ; shift
    local desc="$1"
    local sortie
    sortie=$("$RV" check "$f" 2>&1)
    local code=$?
    if [[ $code -eq 0 ]] ; then
        echo "[KO] $desc (le compilateur aurait dû refuser)"
        ECHEC=$((ECHEC+1))
        return
    fi
    if [[ "$sortie" != *"$fragment"* ]] ; then
        echo "[KO] $desc (message inattendu : $sortie)"
        ECHEC=$((ECHEC+1))
        return
    fi
    echo "[OK] $desc"
}

echo "=== RUSTAVA — non-régression ==="
attend "$DIR/exemples/nexus.rv"          "Aissa"     "l'exemple Nexus exact de l'utilisateur"
attend "$DIR/exemples/test-mariage.rv"   "5050"      "le contrat complet du mariage"
attend "$DIR/exemples/animaux.rv"        "miaou"     "interfaces + dispatch dynamique"
attend "$DIR/exemples/tri.rv"            "1, 2, 3, 5, 7, 8, 9" "le tri à bulles"
attend "$DIR/exemples/concurrent.rv"     "tous les threads ont fini" "spawn : threads légers"

refuse "/tmp/tests-rv/refuse-extends.rv"    "refuse l'héritage"        "extends est refusé (compositions only)"
refuse "/tmp/tests-rv/refuse-let.rv"       "est immuable"             "modifier un let est refusé"
refuse "/tmp/tests-rv/refuse-incomplet.rv" "contrat incomplet"        "interface incomplète est refusée"
refuse "/tmp/tests-rv/refuse-affect.rv"    "attendais"                "affectation dans un if est refusée"

if [[ $ECHEC -eq 0 ]] ; then
    echo "=== TOUS LES TESTS PASSENT ==="
else
    echo "=== $ECHEC ÉCHEC(S) ==="
    exit 1
fi
