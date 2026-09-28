-- POS staff role (distinct from the cloud/terrain role in profiles.role)
alter table public.profiles add column pos_role text check (pos_role in ('admin','manager','caissier'));

create or replace function public.has_pos_role(target_role text)
returns boolean
language sql
security definer
stable
set search_path = public
as $$
  select coalesce((select pos_role = target_role from public.profiles where id = auth.uid() and actif = true), false)
$$;

revoke execute on function public.has_pos_role(text) from public, anon;
grant execute on function public.has_pos_role(text) to authenticated;

create or replace function public.can_manage_pos()
returns boolean
language sql
security definer
stable
set search_path = public
as $$
  select coalesce(
    (select is_super_admin or role = 'admin_tenant' or pos_role in ('admin','manager')
     from public.profiles where id = auth.uid() and actif = true),
    false
  )
$$;

revoke execute on function public.can_manage_pos() from public, anon;
grant execute on function public.can_manage_pos() to authenticated;

-- categories
create table public.categories (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  nom text not null,
  description text,
  created_at timestamptz default now(),
  created_by uuid references public.profiles(id),
  updated_at timestamptz,
  updated_by uuid references public.profiles(id),
  unique (tenant_id, nom)
);

-- fournisseurs
create table public.fournisseurs (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  nom text not null,
  adresse text,
  telephone text,
  ice text,
  email text,
  created_at timestamptz default now(),
  created_by uuid references public.profiles(id),
  updated_at timestamptz,
  updated_by uuid references public.profiles(id)
);

-- articles
create table public.articles (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  code_barre text,
  designation text not null,
  description text,
  image_url text,
  prix_achat numeric not null default 0,
  prix_vente numeric not null default 0,
  prix_grossiste numeric,
  tva numeric not null default 0 check (tva >= 0 and tva <= 100),
  stock numeric not null default 0,
  stock_alerte numeric not null default 0,
  categorie_id uuid references public.categories(id),
  fournisseur_id uuid references public.fournisseurs(id),
  actif boolean not null default true,
  divers_taux numeric not null default 0,
  suivi_lot boolean not null default false,
  est_kit boolean not null default false,
  created_at timestamptz default now(),
  created_by uuid references public.profiles(id),
  updated_at timestamptz,
  updated_by uuid references public.profiles(id)
);

create unique index idx_articles_code_barre_unique on public.articles(tenant_id, code_barre) where code_barre is not null and code_barre != '';

-- article_variantes
create table public.article_variantes (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  article_id uuid not null references public.articles(id),
  taille text,
  couleur text,
  stock_dedie numeric not null default 0,
  code_barre text
);

create unique index idx_variantes_code_barre_unique on public.article_variantes(tenant_id, code_barre) where code_barre is not null and code_barre != '';

-- article_composants (kits)
create table public.article_composants (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  article_id uuid not null references public.articles(id),
  composant_id uuid not null references public.articles(id),
  quantite numeric not null default 1
);

-- article_stocks (per-store quantity)
create table public.article_stocks (
  tenant_id uuid not null references public.tenants(id),
  article_id uuid not null references public.articles(id),
  magasin_id uuid not null references public.magasins(id),
  quantite numeric not null default 0,
  primary key (article_id, magasin_id)
);

-- article_variante_stocks
create table public.article_variante_stocks (
  tenant_id uuid not null references public.tenants(id),
  variante_id uuid not null references public.article_variantes(id),
  magasin_id uuid not null references public.magasins(id),
  quantite numeric not null default 0,
  primary key (variante_id, magasin_id)
);

-- article_lots (FEFO batches with expiry)
create table public.article_lots (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  article_id uuid not null references public.articles(id),
  magasin_id uuid not null references public.magasins(id),
  numero_lot text,
  date_peremption date,
  quantite numeric not null default 0,
  date_reception timestamptz not null default now()
);

-- tenant-consistency triggers: every FK on these tables must belong to the same tenant
create or replace function public.check_catalogue_tenant_consistency()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  if TG_TABLE_NAME = 'articles' then
    if new.categorie_id is not null and not exists (select 1 from public.categories c where c.id = new.categorie_id and c.tenant_id = new.tenant_id) then
      raise exception 'categorie_id n''appartient pas à ce tenant';
    end if;
    if new.fournisseur_id is not null and not exists (select 1 from public.fournisseurs f where f.id = new.fournisseur_id and f.tenant_id = new.tenant_id) then
      raise exception 'fournisseur_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME in ('article_variantes', 'article_composants') then
    if not exists (select 1 from public.articles a where a.id = new.article_id and a.tenant_id = new.tenant_id) then
      raise exception 'article_id n''appartient pas à ce tenant';
    end if;
    if TG_TABLE_NAME = 'article_composants' and not exists (select 1 from public.articles a where a.id = new.composant_id and a.tenant_id = new.tenant_id) then
      raise exception 'composant_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'article_stocks' then
    if not exists (select 1 from public.articles a where a.id = new.article_id and a.tenant_id = new.tenant_id) then
      raise exception 'article_id n''appartient pas à ce tenant';
    end if;
    if not exists (select 1 from public.magasins m where m.id = new.magasin_id and m.tenant_id = new.tenant_id) then
      raise exception 'magasin_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'article_variante_stocks' then
    if not exists (select 1 from public.article_variantes v where v.id = new.variante_id and v.tenant_id = new.tenant_id) then
      raise exception 'variante_id n''appartient pas à ce tenant';
    end if;
    if not exists (select 1 from public.magasins m where m.id = new.magasin_id and m.tenant_id = new.tenant_id) then
      raise exception 'magasin_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'article_lots' then
    if not exists (select 1 from public.articles a where a.id = new.article_id and a.tenant_id = new.tenant_id) then
      raise exception 'article_id n''appartient pas à ce tenant';
    end if;
    if not exists (select 1 from public.magasins m where m.id = new.magasin_id and m.tenant_id = new.tenant_id) then
      raise exception 'magasin_id n''appartient pas à ce tenant';
    end if;
  end if;
  return new;
end;
$$;

revoke execute on function public.check_catalogue_tenant_consistency() from public, anon, authenticated;

create trigger check_articles_tenant_consistency before insert or update on public.articles
  for each row execute function public.check_catalogue_tenant_consistency();
create trigger check_article_variantes_tenant_consistency before insert or update on public.article_variantes
  for each row execute function public.check_catalogue_tenant_consistency();
create trigger check_article_composants_tenant_consistency before insert or update on public.article_composants
  for each row execute function public.check_catalogue_tenant_consistency();
create trigger check_article_stocks_tenant_consistency before insert or update on public.article_stocks
  for each row execute function public.check_catalogue_tenant_consistency();
create trigger check_article_variante_stocks_tenant_consistency before insert or update on public.article_variante_stocks
  for each row execute function public.check_catalogue_tenant_consistency();
create trigger check_article_lots_tenant_consistency before insert or update on public.article_lots
  for each row execute function public.check_catalogue_tenant_consistency();

-- RLS: read open to every active tenant member (needed for the sale screen); writes restricted to admin/manager
alter table public.categories enable row level security;
alter table public.fournisseurs enable row level security;
alter table public.articles enable row level security;
alter table public.article_variantes enable row level security;
alter table public.article_composants enable row level security;
alter table public.article_stocks enable row level security;
alter table public.article_variante_stocks enable row level security;
alter table public.article_lots enable row level security;

create policy categories_select on public.categories for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy categories_write on public.categories for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy fournisseurs_select on public.fournisseurs for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy fournisseurs_write on public.fournisseurs for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy articles_select on public.articles for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy articles_write on public.articles for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy article_variantes_select on public.article_variantes for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy article_variantes_write on public.article_variantes for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy article_composants_select on public.article_composants for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy article_composants_write on public.article_composants for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy article_stocks_select on public.article_stocks for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy article_stocks_write on public.article_stocks for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy article_variante_stocks_select on public.article_variante_stocks for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy article_variante_stocks_write on public.article_variante_stocks for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy article_lots_select on public.article_lots for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy article_lots_write on public.article_lots for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

-- indexes
create index idx_categories_tenant on public.categories(tenant_id);
create index idx_fournisseurs_tenant on public.fournisseurs(tenant_id);
create index idx_articles_tenant on public.articles(tenant_id);
create index idx_articles_code_barre on public.articles(tenant_id, code_barre);
create index idx_articles_actif on public.articles(tenant_id, actif);
create index idx_articles_categorie on public.articles(categorie_id);
create index idx_article_variantes_tenant on public.article_variantes(tenant_id);
create index idx_article_variantes_article on public.article_variantes(article_id);
create index idx_article_composants_tenant on public.article_composants(tenant_id);
create index idx_article_composants_article on public.article_composants(article_id);
create index idx_article_stocks_tenant on public.article_stocks(tenant_id);
create index idx_article_stocks_magasin on public.article_stocks(magasin_id);
create index idx_article_variante_stocks_tenant on public.article_variante_stocks(tenant_id);
create index idx_article_lots_tenant on public.article_lots(tenant_id);
create index idx_article_lots_article on public.article_lots(article_id);
create index idx_article_lots_peremption on public.article_lots(date_peremption);
