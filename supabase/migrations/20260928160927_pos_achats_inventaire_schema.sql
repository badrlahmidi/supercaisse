-- achats
create table public.achats (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  date timestamptz not null default now(),
  fournisseur_id uuid references public.fournisseurs(id),
  reference text,
  montant_total numeric not null default 0,
  statut text default 'recu',
  statut_livraison text not null default 'recu' check (statut_livraison in ('en_attente','partiel','recu')),
  statut_paiement text not null default 'non_paye' check (statut_paiement in ('non_paye','partiel','paye'))
);

-- achat_articles
create table public.achat_articles (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  achat_id uuid not null references public.achats(id) on delete cascade,
  article_id uuid not null references public.articles(id),
  quantite numeric not null,
  prix_unitaire numeric not null,
  total_ligne numeric not null
);

-- inventaires
create table public.inventaires (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  date_debut timestamptz not null default now(),
  date_fin timestamptz,
  statut text not null default 'en_cours' check (statut in ('en_cours','valide')),
  magasin_id uuid not null references public.magasins(id),
  utilisateur_id uuid references public.profiles(id)
);

-- inventaire_lignes
create table public.inventaire_lignes (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  inventaire_id uuid not null references public.inventaires(id) on delete cascade,
  article_id uuid not null references public.articles(id),
  stock_theorique numeric not null default 0,
  stock_compte numeric,
  ecart numeric
);

-- transferts_stock
create table public.transferts_stock (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  source_id uuid not null references public.magasins(id),
  dest_id uuid not null references public.magasins(id),
  date timestamptz not null default now(),
  statut text not null default 'en_attente' check (statut in ('en_attente','valide')),
  utilisateur_id uuid references public.profiles(id)
);

-- transfert_lignes
create table public.transfert_lignes (
  tenant_id uuid not null references public.tenants(id),
  transfert_id uuid not null references public.transferts_stock(id) on delete cascade,
  article_id uuid not null references public.articles(id),
  quantite numeric not null,
  primary key (transfert_id, article_id)
);

-- tenant-consistency triggers
create or replace function public.check_achats_inventaire_tenant_consistency()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  if TG_TABLE_NAME = 'achats' then
    if new.fournisseur_id is not null and not exists (select 1 from public.fournisseurs f where f.id = new.fournisseur_id and f.tenant_id = new.tenant_id) then
      raise exception 'fournisseur_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'achat_articles' then
    if not exists (select 1 from public.achats a where a.id = new.achat_id and a.tenant_id = new.tenant_id) then
      raise exception 'achat_id n''appartient pas à ce tenant';
    end if;
    if not exists (select 1 from public.articles a where a.id = new.article_id and a.tenant_id = new.tenant_id) then
      raise exception 'article_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'inventaires' then
    if not exists (select 1 from public.magasins m where m.id = new.magasin_id and m.tenant_id = new.tenant_id) then
      raise exception 'magasin_id n''appartient pas à ce tenant';
    end if;
    if new.utilisateur_id is not null and not exists (select 1 from public.profiles p where p.id = new.utilisateur_id and p.tenant_id = new.tenant_id) then
      raise exception 'utilisateur_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'inventaire_lignes' then
    if not exists (select 1 from public.inventaires i where i.id = new.inventaire_id and i.tenant_id = new.tenant_id) then
      raise exception 'inventaire_id n''appartient pas à ce tenant';
    end if;
    if not exists (select 1 from public.articles a where a.id = new.article_id and a.tenant_id = new.tenant_id) then
      raise exception 'article_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'transferts_stock' then
    if not exists (select 1 from public.magasins m where m.id = new.source_id and m.tenant_id = new.tenant_id) then
      raise exception 'source_id n''appartient pas à ce tenant';
    end if;
    if not exists (select 1 from public.magasins m where m.id = new.dest_id and m.tenant_id = new.tenant_id) then
      raise exception 'dest_id n''appartient pas à ce tenant';
    end if;
    if new.utilisateur_id is not null and not exists (select 1 from public.profiles p where p.id = new.utilisateur_id and p.tenant_id = new.tenant_id) then
      raise exception 'utilisateur_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'transfert_lignes' then
    if not exists (select 1 from public.transferts_stock t where t.id = new.transfert_id and t.tenant_id = new.tenant_id) then
      raise exception 'transfert_id n''appartient pas à ce tenant';
    end if;
    if not exists (select 1 from public.articles a where a.id = new.article_id and a.tenant_id = new.tenant_id) then
      raise exception 'article_id n''appartient pas à ce tenant';
    end if;
  end if;
  return new;
end;
$$;

revoke execute on function public.check_achats_inventaire_tenant_consistency() from public, anon, authenticated;

create trigger check_achats_tenant_consistency before insert or update on public.achats for each row execute function public.check_achats_inventaire_tenant_consistency();
create trigger check_achat_articles_tenant_consistency before insert or update on public.achat_articles for each row execute function public.check_achats_inventaire_tenant_consistency();
create trigger check_inventaires_tenant_consistency before insert or update on public.inventaires for each row execute function public.check_achats_inventaire_tenant_consistency();
create trigger check_inventaire_lignes_tenant_consistency before insert or update on public.inventaire_lignes for each row execute function public.check_achats_inventaire_tenant_consistency();
create trigger check_transferts_stock_tenant_consistency before insert or update on public.transferts_stock for each row execute function public.check_achats_inventaire_tenant_consistency();
create trigger check_transfert_lignes_tenant_consistency before insert or update on public.transfert_lignes for each row execute function public.check_achats_inventaire_tenant_consistency();

-- RLS: this whole domain is admin/manager only by default (caissier has no module permission here)
alter table public.achats enable row level security;
alter table public.achat_articles enable row level security;
alter table public.inventaires enable row level security;
alter table public.inventaire_lignes enable row level security;
alter table public.transferts_stock enable row level security;
alter table public.transfert_lignes enable row level security;

create policy achats_select on public.achats for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy achats_write on public.achats for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy achat_articles_select on public.achat_articles for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy achat_articles_write on public.achat_articles for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy inventaires_select on public.inventaires for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy inventaires_write on public.inventaires for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy inventaire_lignes_select on public.inventaire_lignes for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy inventaire_lignes_write on public.inventaire_lignes for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy transferts_stock_select on public.transferts_stock for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy transferts_stock_write on public.transferts_stock for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy transfert_lignes_select on public.transfert_lignes for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy transfert_lignes_write on public.transfert_lignes for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

-- indexes
create index idx_achats_tenant on public.achats(tenant_id);
create index idx_achats_date on public.achats(tenant_id, date);
create index idx_achats_fournisseur on public.achats(fournisseur_id);
create index idx_achat_articles_tenant on public.achat_articles(tenant_id);
create index idx_achat_articles_achat on public.achat_articles(achat_id);
create index idx_achat_articles_article on public.achat_articles(article_id);
create index idx_inventaires_tenant on public.inventaires(tenant_id);
create index idx_inventaires_magasin on public.inventaires(magasin_id, statut);
create index idx_inventaire_lignes_tenant on public.inventaire_lignes(tenant_id);
create index idx_inventaire_lignes_inventaire on public.inventaire_lignes(inventaire_id);
create index idx_transferts_stock_tenant on public.transferts_stock(tenant_id);
create index idx_transfert_lignes_tenant on public.transfert_lignes(tenant_id);
