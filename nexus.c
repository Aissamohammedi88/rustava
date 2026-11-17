#include "runtime.h"

// ----------------------------------------
// classe Nexus
// ----------------------------------------
typedef struct {
    RV_TETE_OBJET;
} Nexus;

static void fn_main(void);
static void m_Nexus_main_(Nexus *self);
static void m_Nexus_afficher_(Nexus *self, RvStr* rv_texte);

static const RvMeth rv_meths_Nexus[] = {
    { "main", (void*)m_Nexus_main_ },
    { "afficher", (void*)m_Nexus_afficher_ },
};
static const RvClasse rv_desc_Nexus = { "Nexus", rv_meths_Nexus, 2 };

// fabrique de Nexus
static RvObj* new_Nexus(void) {
    Nexus* o = (Nexus*)rv_objet_creer(sizeof(Nexus));
    o->rv_classe = &rv_desc_Nexus;
    return (RvObj*)o;
}

static void fn_main(void) {
    RvObj* rv_n = new_Nexus();
    (void)m_Nexus_main_(((Nexus*)rv_n));
}

static void m_Nexus_main_(Nexus *self) {
    RvStr* rv_nom = rv_str_lit("Aissa");
    int64_t rv_age = 30;
    (void)rv_afficher_str(rv_nom);
    rv_age = (rv_age + 1);
    (void)rv_afficher_int(rv_age);
}

static void m_Nexus_afficher_(Nexus *self, RvStr* rv_texte) {
    (void)rv_afficher_str(rv_texte);
}

int main(void) {
    rv_runtime_init();
    RV_TRY {
        fn_main();
    } else {
        RvStr* e = rv_erreur_active;
        rv_erreur_non_attrapee(e);
    }
    return 0;
}
