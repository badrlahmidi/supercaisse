create extension if not exists pgcrypto;

create table public.tenants (
  id uuid primary key default gen_random_uuid(),
  nom text,
  slug text unique,
  statut text default 'actif',
  plan text,
  cree_le timestamptz default now()
);

create table public.profiles (
  id uuid primary key references auth.users(id),
  tenant_id uuid references public.tenants(id),
  nom text,
  role text check (role in ('admin_tenant','commercial','receptionniste','manager_cloud')),
  magasin_id uuid,
  is_super_admin boolean default false,
  actif boolean default true
);

create table public.magasins (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  nom text,
  adresse text,
  magasin_local_id text
);

create table public.articles_cache (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  magasin_id uuid references public.magasins(id),
  article_local_id text,
  designation text,
  prix_vente numeric,
  stock_indicatif numeric,
  actif boolean default true,
  maj_le timestamptz default now()
);

create table public.commande_terrain (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  magasin_id uuid references public.magasins(id),
  commercial_id uuid references public.profiles(id),
  client_nom text,
  client_telephone text,
  statut text default 'brouillon' check (statut in ('brouillon','soumise','validee','refusee','en_preparation','prete','livree','annulee')),
  notes text,
  latitude double precision,
  longitude double precision,
  date_creation timestamptz default now(),
  date_soumission timestamptz,
  date_validation timestamptz,
  date_preparation timestamptz,
  date_livraison timestamptz,
  valide_par uuid references public.profiles(id),
  prepare_par uuid references public.profiles(id)
);

create table public.commande_terrain_ligne (
  id uuid primary key default gen_random_uuid(),
  commande_id uuid references public.commande_terrain(id) on delete cascade,
  article_local_id text,
  designation text,
  quantite numeric,
  prix_propose numeric,
  remise numeric default 0
);

create table public.commande_terrain_historique (
  id uuid primary key default gen_random_uuid(),
  commande_id uuid references public.commande_terrain(id) on delete cascade,
  statut text,
  utilisateur_id uuid references public.profiles(id),
  commentaire text,
  date timestamptz default now()
);

create table public.evenements_business (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  magasin_id uuid references public.magasins(id),
  type text,
  payload jsonb,
  date timestamptz default now()
);
