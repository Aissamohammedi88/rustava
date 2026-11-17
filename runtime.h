/* ============================================================
   RUSTAVA — runtime C (le remplacement de la JVM)
   ------------------------------------------------------------
   Le mariage, côté mémoire :
     • ZÉRO ramasse-miettes : aucune pause, aucun gel.
     • Arène déterministe : tout est alloué dans un seul bloc
       d'allocations, rendu d'un coup à la fin du programme
       (enregistré via atexit : même sur les erreurs).
     • Exceptions try/catch : setjmp/longjmp.
     • spawn : threads légers détachés (pthreads).
   ============================================================ */

#ifndef RV_RUNTIME_H
#define RV_RUNTIME_H

/* POSIX : clock_gettime, nanosleep (rendus visibles même en -std=c11) */
#define _POSIX_C_SOURCE 199309L

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <stdint.h>
#include <inttypes.h>
#include <setjmp.h>
#include <pthread.h>
#include <time.h>
#include <math.h>

/* ---------- le descripteur de classe (vtable) ---------- */

typedef struct RvClasse RvClasse;
typedef struct RvMeth { const char* nom; void* fn; } RvMeth;
struct RvClasse {
    const char* nom;
    const RvMeth* meths;
    int nb;
};

/* tout objet Rustava commence par son descripteur */
typedef struct RvObj { const RvClasse* rv_classe; } RvObj;
#define RV_TETE_OBJET const RvClasse* rv_classe;

/* ---------- l'arène (mémoire déterministe) ---------- */

#define RV_MAX_ALLOCS 262144

static void* rv_arena[RV_MAX_ALLOCS];
static int   rv_arena_n = 0;

static void* rv_allouer(size_t taille) {
    if (rv_arena_n >= RV_MAX_ALLOCS) {
        fprintf(stderr, "\nErreur Rustava : trop d'allocations (> %d)\n", RV_MAX_ALLOCS);
        exit(1);
    }
    void* p = calloc(1, taille ? taille : 1);
    if (!p) {
        fprintf(stderr, "\nErreur Rustava : plus de mémoire\n");
        exit(1);
    }
    rv_arena[rv_arena_n++] = p;
    return p;
}

static void rv_arena_vider(void) {
    for (int i = 0; i < rv_arena_n; i++) free(rv_arena[i]);
    rv_arena_n = 0;
}

static void rv_runtime_init(void) {
    static int deja = 0;
    if (!deja) {
        deja = 1;
        atexit(rv_arena_vider);
    }
}

/* ---------- chaînes (préalables aux exceptions) ---------- */

typedef struct RvStr { RV_TETE_OBJET; char* t; int64_t n; } RvStr;

static const RvClasse rv_classe_str = { "string", 0, 0 };

static RvStr* rv_str_fabrique(const char* s, int64_t n) {
    RvStr* r = (RvStr*)rv_allouer(sizeof(RvStr));
    r->rv_classe = &rv_classe_str;
    r->t = (char*)rv_allouer((size_t)n + 1);
    memcpy(r->t, s, (size_t)n);
    r->t[n] = '\0';
    r->n = n;
    return r;
}

static RvStr* rv_str_lit(const char* s) {
    return rv_str_fabrique(s, (int64_t)strlen(s));
}

/* ---------- exceptions (try / catch) ----------
   Le catch est généré EXPLICITEMENT par le compilateur :
     RV_TRY { corps } else { RvStr* e = rv_erreur_active; rv_pile_prof--; ... }
   (pas de macro RV_CATCH : les accolades restent lisibles)      */

#define RV_MAX_PILE 256
static jmp_buf rv_pile[RV_MAX_PILE];
static int     rv_pile_prof = 0;
static RvStr*  rv_erreur_active = NULL;

#define RV_TRY if (setjmp(rv_pile[rv_pile_prof++]) == 0)

static void rv_throw(RvStr* s) {
    rv_erreur_active = s;
    if (rv_pile_prof > 0) longjmp(rv_pile[rv_pile_prof - 1], 1);
    fprintf(stderr, "\nErreur Rustava non attrapée : %s\n", s && s->t ? s->t : "?");
    exit(1);
}

static void rv_throw_msg(const char* msg) {
    rv_throw(rv_str_lit(msg));
}

#define RV_THROW(s) rv_throw(s)

static void rv_erreur_non_attrapee(RvStr* e) {
    fprintf(stderr, "\nErreur Rustava non attrapée : %s\n", e && e ? (e->t ? e->t : "?") : "?");
}

/* ---------- chaînes : la suite ---------- */

static int64_t rv_longueur_str(RvStr* s) { return s ? s->n : 0; }

static bool rv_str_egal(RvStr* a, RvStr* b) {
    if (!a || !b) return a == b;
    return a->n == b->n && memcmp(a->t, b->t, (size_t)a->n) == 0;
}

static int rv_str_cmp(RvStr* a, RvStr* b) {
    if (!a && !b) return 0;
    if (!a) return -1;
    if (!b) return 1;
    int64_t m = a->n < b->n ? a->n : b->n;
    int c = memcmp(a->t, b->t, (size_t)m);
    if (c) return c;
    if (a->n < b->n) return -1;
    if (a->n > b->n) return 1;
    return 0;
}

static RvStr* rv_str_concat(RvStr* a, RvStr* b) {
    if (!a) return b;
    if (!b) return a;
    int64_t n = a->n + b->n;
    RvStr* r = (RvStr*)rv_allouer(sizeof(RvStr));
    r->rv_classe = &rv_classe_str;
    r->t = (char*)rv_allouer((size_t)n + 1);
    memcpy(r->t, a->t, (size_t)a->n);
    memcpy(r->t + a->n, b->t, (size_t)b->n);
    r->t[n] = '\0';
    r->n = n;
    return r;
}

static RvStr* rv_str_de_int(int64_t v) {
    char buf[32];
    snprintf(buf, sizeof buf, "%" PRId64, v);
    return rv_str_fabrique(buf, (int64_t)strlen(buf));
}

static int64_t rv_str_vers_int(RvStr* s) {
    if (!s || !s->t) return 0;
    return strtoll(s->t, NULL, 10);
}

static RvStr* rv_str_majuscule(RvStr* s) {
    if (!s) return rv_str_lit("");
    RvStr* r = (RvStr*)rv_allouer(sizeof(RvStr));
    r->rv_classe = &rv_classe_str;
    r->n = s->n;
    r->t = (char*)rv_allouer((size_t)s->n + 1);
    for (int64_t i = 0; i < s->n; i++) {
        char c = s->t[i];
        r->t[i] = (c >= 'a' && c <= 'z') ? (char)(c - 'a' + 'A') : c;
    }
    r->t[s->n] = '\0';
    return r;
}

static RvStr* rv_caractere(RvStr* s, int64_t i) {
    if (!s || i < 0 || i >= s->n) {
        rv_throw_msg("caractere(s, i) : indice hors limites");
    }
    return rv_str_fabrique(s->t + i, 1);
}

/* ---------- les tableaux ---------- */

typedef union RVVal { int64_t i; double f; void* p; } RVVal;

typedef enum { RV_EL_I64 = 0, RV_EL_F64, RV_EL_STR, RV_EL_OBJ } RvElem;

typedef struct RvArr { RV_TETE_OBJET; RvElem el; int64_t n; RVVal* v; } RvArr;

static const RvClasse rv_classe_arr = { "tableau", 0, 0 };

static RVVal rv_v_i64(int64_t x) { RVVal v; v.i = x; return v; }
static RVVal rv_v_f64(double x)  { RVVal v; v.f = x; return v; }
static RVVal rv_v_str(RvStr* x)  { RVVal v; v.p = x; return v; }
static RVVal rv_v_obj(RvObj* x)  { RVVal v; v.p = x; return v; }

static RvArr* rv_arr_creer(RvElem el, int64_t cap) {
    if (cap < 0) cap = 0;
    RvArr* a = (RvArr*)rv_allouer(sizeof(RvArr));
    a->rv_classe = &rv_classe_arr;
    a->el = el;
    a->n = 0;
    a->v = (RVVal*)rv_allouer(sizeof(RVVal) * (size_t)(cap ? cap : 1));
    for (int64_t i = 0; i < cap; i++) a->v[i].i = 0;
    return a;
}

static RvArr* rv_arr_lit(RvElem el, int64_t n, RVVal* vals) {
    RvArr* a = rv_arr_creer(el, n);
    for (int64_t i = 0; i < n; i++) a->v[i] = vals[i];
    a->n = n;
    return a;
}

static int64_t rv_arr_len(RvArr* a) { return a ? a->n : 0; }

static void rv_verif_indice(RvArr* a, int64_t i) {
    if (!a || i < 0 || i >= a->n) {
        char buf[96];
        snprintf(buf, sizeof buf,
                 "indice hors limites (taille %lld, indice %lld)",
                 (long long)(a ? a->n : 0), (long long)i);
        rv_throw_msg(buf);
    }
}

static int64_t rv_arr_lire_i64(RvArr* a, int64_t i) { rv_verif_indice(a, i); return a->v[i].i; }
static double  rv_arr_lire_f64(RvArr* a, int64_t i) { rv_verif_indice(a, i); return a->v[i].f; }
static RvStr*  rv_arr_lire_str(RvArr* a, int64_t i) { rv_verif_indice(a, i); return (RvStr*)a->v[i].p; }
static RvObj*  rv_arr_lire_obj(RvArr* a, int64_t i) { rv_verif_indice(a, i); return (RvObj*)a->v[i].p; }

static void rv_arr_ecrire_i64(RvArr* a, int64_t i, const char* op, int64_t x) {
    rv_verif_indice(a, i);
    if (!strcmp(op, "=")) a->v[i].i = x;
    else if (!strcmp(op, "+=")) a->v[i].i += x;
    else if (!strcmp(op, "-=")) a->v[i].i -= x;
    else if (!strcmp(op, "*=")) a->v[i].i *= x;
    else a->v[i].i /= x;
}
static void rv_arr_ecrire_f64(RvArr* a, int64_t i, const char* op, double x) {
    rv_verif_indice(a, i);
    if (!strcmp(op, "=")) a->v[i].f = x;
    else if (!strcmp(op, "+=")) a->v[i].f += x;
    else if (!strcmp(op, "-=")) a->v[i].f -= x;
    else if (!strcmp(op, "*=")) a->v[i].f *= x;
    else a->v[i].f /= x;
}
static void rv_arr_ecrire_str(RvArr* a, int64_t i, const char* op, RvStr* x) {
    rv_verif_indice(a, i);
    if (!strcmp(op, "+=")) { a->v[i].p = rv_str_concat((RvStr*)a->v[i].p, x); }
    else a->v[i].p = x;
}

/* ---------- afficher ---------- */

static void rv_afficher_int(int64_t v)    { printf("%" PRId64 "\n", v); }
static void rv_afficher_float(double v)   { printf("%g\n", v); }
static void rv_afficher_bool(bool v)      { printf(v ? "vrai\n" : "faux\n"); }
static void rv_afficher_str(RvStr* s)     { printf("%s\n", s && s->t ? s->t : ""); }

static void rv_afficher_obj(RvObj* o) {
    if (!o) { printf("null\n"); return; }
    if (o->rv_classe == &rv_classe_str) {
        printf("%s\n", ((RvStr*)o)->t ? ((RvStr*)o)->t : "");
        return;
    }
    if (o->rv_classe == &rv_classe_arr) {
        RvArr* a = (RvArr*)o;
        printf("[");
        for (int64_t i = 0; i < a->n; i++) {
            if (i) printf(", ");
            switch (a->el) {
                case RV_EL_I64: printf("%" PRId64, a->v[i].i); break;
                case RV_EL_F64: printf("%g", a->v[i].f); break;
                case RV_EL_STR: printf("\"%s\"", ((RvStr*)a->v[i].p)->t); break;
                case RV_EL_OBJ: printf("<%s>", ((RvObj*)a->v[i].p)->rv_classe->nom); break;
            }
        }
        printf("]\n");
        return;
    }
    printf("<%s>\n", o->rv_classe->nom);
}

/* rv_longueur : string OU tableau (dispatch par descripteur) */
static int64_t rv_longueur(RvObj* x) {
    if (!x) return 0;
    if (x->rv_classe == &rv_classe_arr) return ((RvArr*)x)->n;
    if (x->rv_classe == &rv_classe_str) return ((RvStr*)x)->n;
    return 0;
}

/* ---------- division protégée (comme Rust : panique → throw attrapable) ---------- */

static int64_t rv_div_i64(int64_t a, int64_t b) {
    if (b == 0) rv_throw_msg("division par zéro");
    return a / b;
}
static double rv_div_f64(double a, double b) {
    if (b == 0.0) rv_throw_msg("division par zéro");
    return a / b;
}
static int64_t rv_mod_i64(int64_t a, int64_t b) {
    if (b == 0) rv_throw_msg("modulo par zéro");
    return a % b;
}
static double rv_mod_f64(double a, double b) {
    if (b == 0.0) rv_throw_msg("modulo par zéro");
    return fmod(a, b);
}

/* ---------- spawn : threads légers détachés ---------- */

typedef struct { void (*fn)(void); } RvJob;

static void* rv_trampoline(void* p) {
    RvJob* j = (RvJob*)p;
    j->fn();
    free(j);
    return NULL;
}

static void rv_spawn_0(void (*fn)(void)) {
    RvJob* j = (RvJob*)malloc(sizeof(RvJob));
    if (!j) return;
    j->fn = fn;
    pthread_t t;
    if (pthread_create(&t, NULL, rv_trampoline, j) == 0) {
        pthread_detach(t);
    } else {
        j->fn();
        free(j);
    }
}

/* spawn avec 1 argument int (les littéraux et petites valeurs) */

typedef struct { void (*fn)(int64_t); int64_t a; } RvJobI;

static void* rv_trampoline_i(void* p) {
    RvJobI* j = (RvJobI*)p;
    j->fn(j->a);
    free(j);
    return NULL;
}

static void rv_spawn_i(void (*fn)(int64_t), int64_t a) {
    RvJobI* j = (RvJobI*)malloc(sizeof(RvJobI));
    if (!j) return;
    j->fn = fn;
    j->a = a;
    pthread_t t;
    if (pthread_create(&t, NULL, rv_trampoline_i, j) == 0) {
        pthread_detach(t);
    } else {
        j->fn(a);
        free(j);
    }
}

/* spawn avec 1 argument string */

typedef struct { void (*fn)(RvStr*); RvStr* s; } RvJobS;

static void* rv_trampoline_s(void* p) {
    RvJobS* j = (RvJobS*)p;
    j->fn(j->s);
    free(j);
    return NULL;
}

static void rv_spawn_s(void (*fn)(RvStr*), RvStr* s) {
    RvJobS* j = (RvJobS*)malloc(sizeof(RvJobS));
    if (!j) return;
    j->fn = fn;
    j->s = s;
    pthread_t t;
    if (pthread_create(&t, NULL, rv_trampoline_s, j) == 0) {
        pthread_detach(t);
    } else {
        j->fn(s);
        free(j);
    }
}

/* ---------- horloge ---------- */

static int64_t rv_horloge_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (int64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

static void rv_dormir_ms(int64_t ms) {
    struct timespec ts;
    if (ms < 0) ms = 0;
    ts.tv_sec = (time_t)(ms / 1000);
    ts.tv_nsec = (long)((ms % 1000) * 1000000);
    nanosleep(&ts, NULL);
}

/* ---------- objets et dispatch dynamique ---------- */

static void* rv_objet_creer(size_t taille) {
    return rv_allouer(taille);
}

static void* rv_methode(const RvClasse* c, const char* nom) {
    for (int i = 0; i < c->nb; i++) {
        if (strcmp(c->meths[i].nom, nom) == 0) return c->meths[i].fn;
    }
    fprintf(stderr, "\nErreur interne Rustava : méthode « %s » absente de « %s »\n", nom, c->nom);
    exit(1);
}

#endif /* RV_RUNTIME_H */
