-- clients
create table public.clients (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  code text,
  nom text not null,
  adresse text,
  telephone text,
  email text,
  ice text,
  credit_plafond numeric not null default 0,
  credit_actuel numeric not null default 0,
  points_fidelite numeric not null default 0,
  segment text,
  created_at timestamptz default now(),
  created_by uuid references public.profiles(id),
  updated_at timestamptz,
  updated_by uuid references public.profiles(id)
);

create unique index idx_clients_code_unique on public.clients(tenant_id, code) where code is not null and code != '';

-- sessions_caisse
create table public.sessions_caisse (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  caissier_id uuid not null references public.profiles(id),
  magasin_id uuid references public.magasins(id),
  date_ouverture timestamptz not null default now(),
  date_cloture timestamptz,
  fond_initial numeric not null default 0 check (fond_initial >= 0),
  total_especes_attendu numeric default 0,
  total_especes_declare numeric default 0,
  ecart numeric default 0,
  statut text not null default 'ouverte' check (statut in ('ouverte','cloturee'))
);

-- caisses (physical tills)
create table public.caisses (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  nom text not null,
  utilisateur_id uuid references public.profiles(id),
  statut text not null default 'fermee',
  ouverture_date timestamptz,
  fermeture_date timestamptz,
  fond_initial numeric not null default 0,
  recettes_especes numeric not null default 0,
  recettes_cb numeric not null default 0,
  recettes_cheque numeric not null default 0,
  recettes_virement numeric not null default 0,
  depenses numeric not null default 0,
  ecart numeric not null default 0,
  note text
);

-- ventes
create table public.ventes (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  date timestamptz not null default now(),
  client_id uuid references public.clients(id),
  caissier_id uuid references public.profiles(id),
  magasin_id uuid references public.magasins(id),
  montant_total numeric not null default 0,
  montant_remise numeric not null default 0,
  montant_ht numeric,
  montant_tva numeric,
  mode_paiement text not null default 'especes' check (mode_paiement in ('especes','carte','cb','cheque','virement','credit','fidelite','mixte')),
  statut text not null default 'validee' check (statut in ('validee','annulee','convertie')),
  points_utilises numeric not null default 0 check (points_utilises >= 0),
  points_gagnes numeric not null default 0,
  numero_facture text,
  dtype text not null default 'facture' check (dtype in ('facture','bl','devis','commande','avoir')),
  session_id uuid references public.sessions_caisse(id),
  source_vente_id uuid references public.ventes(id),
  check (dtype = 'avoir' or montant_total >= 0)
);

create unique index idx_ventes_numero_facture_unique on public.ventes(tenant_id, numero_facture) where numero_facture is not null;

-- vente_articles
create table public.vente_articles (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  vente_id uuid not null references public.ventes(id) on delete cascade,
  article_id uuid not null references public.articles(id),
  variante_id uuid references public.article_variantes(id),
  quantite numeric not null check (quantite > 0),
  prix_unitaire numeric not null,
  prix_type text not null default 'public' check (prix_type in ('public','grossiste')),
  tva numeric not null default 0 check (tva >= 0 and tva <= 100),
  remise_ligne numeric not null default 0 check (remise_ligne >= 0 and remise_ligne <= 100),
  total_ligne numeric not null,
  montant_ht numeric,
  montant_tva numeric,
  note text
);

-- vente_paiements (supports split/mixed payments)
create table public.vente_paiements (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  vente_id uuid not null references public.ventes(id) on delete cascade,
  session_id uuid references public.sessions_caisse(id),
  mode text not null check (mode in ('especes','carte','cb','cheque','virement','credit','fidelite')),
  montant numeric not null check (montant >= 0)
);

-- vente_lots (which FEFO batch a sale drew from)
create table public.vente_lots (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  vente_id uuid not null references public.ventes(id) on delete cascade,
  lot_id uuid not null references public.article_lots(id),
  quantite numeric not null
);

-- journal_caisse (cash ledger, append-only)
create table public.journal_caisse (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  date timestamptz not null default now(),
  utilisateur_id uuid references public.profiles(id),
  session_id uuid references public.sessions_caisse(id),
  jtype text not null check (jtype in ('entree','sortie','encaissement')),
  montant numeric not null,
  description text
);

-- cheques
create table public.cheques (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  numero text not null,
  banque text not null,
  tireur text,
  montant numeric not null check (montant >= 0),
  date_emission date not null,
  date_echeance date not null,
  statut text not null default 'en_attente' check (statut in ('en_attente','encaisse','impaye')),
  ctype text not null check (ctype in ('client','fournisseur')),
  client_id uuid references public.clients(id),
  fournisseur_id uuid references public.fournisseurs(id)
);

-- mouvements_stock (append-only ledger)
create table public.mouvements_stock (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  date timestamptz not null default now(),
  article_id uuid not null references public.articles(id),
  magasin_id uuid references public.magasins(id),
  quantite numeric not null,
  mtype text not null check (mtype in ('entree','sortie','inventaire')),
  reference_id uuid,
  reference_type text
);

-- mouvements_fidelite (append-only ledger)
create table public.mouvements_fidelite (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  client_id uuid not null references public.clients(id),
  vente_id uuid references public.ventes(id),
  points numeric not null,
  mtype text not null check (mtype in ('gain','depense')),
  date timestamptz not null default now()
);

-- numerotation_v2: legal sequential document numbering, per tenant/type/year.
-- Locked down: no direct client policy at all, only next_document_number() below may write.
create table public.numerotation_v2 (
  tenant_id uuid not null references public.tenants(id),
  ntype text not null,
  annee integer not null,
  prefixe text not null,
  dernier_numero integer not null default 0,
  primary key (tenant_id, ntype, annee)
);

create or replace function public.next_document_number(p_tenant_id uuid, p_ntype text, p_prefixe text)
returns text
language plpgsql
security definer
set search_path = public
as $$
declare
  v_annee integer := extract(year from now())::integer;
  v_numero integer;
begin
  if not (p_tenant_id = public.current_tenant_id() or public.is_super_admin()) then
    raise exception 'Non autorisé';
  end if;

  insert into public.numerotation_v2 (tenant_id, ntype, annee, prefixe, dernier_numero)
  values (p_tenant_id, p_ntype, v_annee, p_prefixe, 1)
  on conflict (tenant_id, ntype, annee)
  do update set dernier_numero = public.numerotation_v2.dernier_numero + 1
  returning dernier_numero into v_numero;

  return p_prefixe || '-' || v_annee || '-' || lpad(v_numero::text, 5, '0');
end;
$$;

revoke execute on function public.next_document_number(uuid, text, text) from public, anon;
grant execute on function public.next_document_number(uuid, text, text) to authenticated;

-- tenant-consistency triggers
create or replace function public.check_ventes_tenant_consistency()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  if TG_TABLE_NAME = 'sessions_caisse' then
    if not exists (select 1 from public.profiles p where p.id = new.caissier_id and p.tenant_id = new.tenant_id) then
      raise exception 'caissier_id n''appartient pas à ce tenant';
    end if;
    if new.magasin_id is not null and not exists (select 1 from public.magasins m where m.id = new.magasin_id and m.tenant_id = new.tenant_id) then
      raise exception 'magasin_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'caisses' then
    if new.utilisateur_id is not null and not exists (select 1 from public.profiles p where p.id = new.utilisateur_id and p.tenant_id = new.tenant_id) then
      raise exception 'utilisateur_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'ventes' then
    if new.client_id is not null and not exists (select 1 from public.clients c where c.id = new.client_id and c.tenant_id = new.tenant_id) then
      raise exception 'client_id n''appartient pas à ce tenant';
    end if;
    if new.caissier_id is not null and not exists (select 1 from public.profiles p where p.id = new.caissier_id and p.tenant_id = new.tenant_id) then
      raise exception 'caissier_id n''appartient pas à ce tenant';
    end if;
    if new.magasin_id is not null and not exists (select 1 from public.magasins m where m.id = new.magasin_id and m.tenant_id = new.tenant_id) then
      raise exception 'magasin_id n''appartient pas à ce tenant';
    end if;
    if new.session_id is not null and not exists (select 1 from public.sessions_caisse s where s.id = new.session_id and s.tenant_id = new.tenant_id) then
      raise exception 'session_id n''appartient pas à ce tenant';
    end if;
    if new.source_vente_id is not null and not exists (select 1 from public.ventes v where v.id = new.source_vente_id and v.tenant_id = new.tenant_id) then
      raise exception 'source_vente_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'vente_articles' then
    if not exists (select 1 from public.ventes v where v.id = new.vente_id and v.tenant_id = new.tenant_id) then
      raise exception 'vente_id n''appartient pas à ce tenant';
    end if;
    if not exists (select 1 from public.articles a where a.id = new.article_id and a.tenant_id = new.tenant_id) then
      raise exception 'article_id n''appartient pas à ce tenant';
    end if;
    if new.variante_id is not null and not exists (select 1 from public.article_variantes vr where vr.id = new.variante_id and vr.tenant_id = new.tenant_id) then
      raise exception 'variante_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'vente_paiements' then
    if not exists (select 1 from public.ventes v where v.id = new.vente_id and v.tenant_id = new.tenant_id) then
      raise exception 'vente_id n''appartient pas à ce tenant';
    end if;
    if new.session_id is not null and not exists (select 1 from public.sessions_caisse s where s.id = new.session_id and s.tenant_id = new.tenant_id) then
      raise exception 'session_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'vente_lots' then
    if not exists (select 1 from public.ventes v where v.id = new.vente_id and v.tenant_id = new.tenant_id) then
      raise exception 'vente_id n''appartient pas à ce tenant';
    end if;
    if not exists (select 1 from public.article_lots l where l.id = new.lot_id and l.tenant_id = new.tenant_id) then
      raise exception 'lot_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'journal_caisse' then
    if new.utilisateur_id is not null and not exists (select 1 from public.profiles p where p.id = new.utilisateur_id and p.tenant_id = new.tenant_id) then
      raise exception 'utilisateur_id n''appartient pas à ce tenant';
    end if;
    if new.session_id is not null and not exists (select 1 from public.sessions_caisse s where s.id = new.session_id and s.tenant_id = new.tenant_id) then
      raise exception 'session_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'cheques' then
    if new.client_id is not null and not exists (select 1 from public.clients c where c.id = new.client_id and c.tenant_id = new.tenant_id) then
      raise exception 'client_id n''appartient pas à ce tenant';
    end if;
    if new.fournisseur_id is not null and not exists (select 1 from public.fournisseurs f where f.id = new.fournisseur_id and f.tenant_id = new.tenant_id) then
      raise exception 'fournisseur_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'mouvements_stock' then
    if not exists (select 1 from public.articles a where a.id = new.article_id and a.tenant_id = new.tenant_id) then
      raise exception 'article_id n''appartient pas à ce tenant';
    end if;
    if new.magasin_id is not null and not exists (select 1 from public.magasins m where m.id = new.magasin_id and m.tenant_id = new.tenant_id) then
      raise exception 'magasin_id n''appartient pas à ce tenant';
    end if;
  elsif TG_TABLE_NAME = 'mouvements_fidelite' then
    if not exists (select 1 from public.clients c where c.id = new.client_id and c.tenant_id = new.tenant_id) then
      raise exception 'client_id n''appartient pas à ce tenant';
    end if;
    if new.vente_id is not null and not exists (select 1 from public.ventes v where v.id = new.vente_id and v.tenant_id = new.tenant_id) then
      raise exception 'vente_id n''appartient pas à ce tenant';
    end if;
  end if;
  return new;
end;
$$;

revoke execute on function public.check_ventes_tenant_consistency() from public, anon, authenticated;

create trigger check_sessions_caisse_tenant_consistency before insert or update on public.sessions_caisse for each row execute function public.check_ventes_tenant_consistency();
create trigger check_caisses_tenant_consistency before insert or update on public.caisses for each row execute function public.check_ventes_tenant_consistency();
create trigger check_ventes_tenant_consistency before insert or update on public.ventes for each row execute function public.check_ventes_tenant_consistency();
create trigger check_vente_articles_tenant_consistency before insert or update on public.vente_articles for each row execute function public.check_ventes_tenant_consistency();
create trigger check_vente_paiements_tenant_consistency before insert or update on public.vente_paiements for each row execute function public.check_ventes_tenant_consistency();
create trigger check_vente_lots_tenant_consistency before insert or update on public.vente_lots for each row execute function public.check_ventes_tenant_consistency();
create trigger check_journal_caisse_tenant_consistency before insert or update on public.journal_caisse for each row execute function public.check_ventes_tenant_consistency();
create trigger check_cheques_tenant_consistency before insert or update on public.cheques for each row execute function public.check_ventes_tenant_consistency();
create trigger check_mouvements_stock_tenant_consistency before insert or update on public.mouvements_stock for each row execute function public.check_ventes_tenant_consistency();
create trigger check_mouvements_fidelite_tenant_consistency before insert or update on public.mouvements_fidelite for each row execute function public.check_ventes_tenant_consistency();

-- RLS
alter table public.clients enable row level security;
alter table public.sessions_caisse enable row level security;
alter table public.caisses enable row level security;
alter table public.ventes enable row level security;
alter table public.vente_articles enable row level security;
alter table public.vente_paiements enable row level security;
alter table public.vente_lots enable row level security;
alter table public.journal_caisse enable row level security;
alter table public.cheques enable row level security;
alter table public.mouvements_stock enable row level security;
alter table public.mouvements_fidelite enable row level security;
alter table public.numerotation_v2 enable row level security;

create policy clients_select on public.clients for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy clients_write on public.clients for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy sessions_caisse_select on public.sessions_caisse for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy sessions_caisse_insert on public.sessions_caisse for insert
  with check (
    public.is_super_admin()
    or (tenant_id = public.current_tenant_id() and (public.can_manage_pos() or public.has_pos_role('caissier')) and caissier_id = auth.uid())
  );
create policy sessions_caisse_update on public.sessions_caisse for update
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and (public.can_manage_pos() or caissier_id = auth.uid())))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and (public.can_manage_pos() or caissier_id = auth.uid())));

create policy caisses_select on public.caisses for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy caisses_write on public.caisses for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy ventes_select on public.ventes for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy ventes_insert on public.ventes for insert
  with check (
    public.is_super_admin()
    or (tenant_id = public.current_tenant_id() and (public.can_manage_pos() or public.has_pos_role('caissier')))
  );
create policy ventes_update on public.ventes for update
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy vente_articles_select on public.vente_articles for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy vente_articles_insert on public.vente_articles for insert
  with check (
    public.is_super_admin()
    or (tenant_id = public.current_tenant_id() and (public.can_manage_pos() or public.has_pos_role('caissier')))
  );
create policy vente_articles_update on public.vente_articles for update
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy vente_paiements_select on public.vente_paiements for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy vente_paiements_insert on public.vente_paiements for insert
  with check (
    public.is_super_admin()
    or (tenant_id = public.current_tenant_id() and (public.can_manage_pos() or public.has_pos_role('caissier')))
  );
create policy vente_paiements_update on public.vente_paiements for update
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy vente_lots_select on public.vente_lots for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy vente_lots_insert on public.vente_lots for insert
  with check (
    public.is_super_admin()
    or (tenant_id = public.current_tenant_id() and (public.can_manage_pos() or public.has_pos_role('caissier')))
  );

create policy journal_caisse_select on public.journal_caisse for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy journal_caisse_insert on public.journal_caisse for insert
  with check (
    public.is_super_admin()
    or (tenant_id = public.current_tenant_id() and (public.can_manage_pos() or public.has_pos_role('caissier')))
  );

create policy cheques_select on public.cheques for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy cheques_write on public.cheques for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

create policy mouvements_stock_select on public.mouvements_stock for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy mouvements_stock_insert on public.mouvements_stock for insert
  with check (
    public.is_super_admin()
    or (tenant_id = public.current_tenant_id() and (public.can_manage_pos() or public.has_pos_role('caissier')))
  );

create policy mouvements_fidelite_select on public.mouvements_fidelite for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy mouvements_fidelite_insert on public.mouvements_fidelite for insert
  with check (
    public.is_super_admin()
    or (tenant_id = public.current_tenant_id() and (public.can_manage_pos() or public.has_pos_role('caissier')))
  );

create policy numerotation_v2_select on public.numerotation_v2 for select
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_pos()));

-- indexes
create index idx_clients_tenant on public.clients(tenant_id);
create index idx_sessions_caisse_tenant on public.sessions_caisse(tenant_id);
create index idx_sessions_caissier on public.sessions_caisse(caissier_id, statut);
create index idx_caisses_tenant on public.caisses(tenant_id);
create index idx_ventes_tenant on public.ventes(tenant_id);
create index idx_ventes_date on public.ventes(tenant_id, date);
create index idx_ventes_client on public.ventes(client_id);
create index idx_ventes_caissier on public.ventes(caissier_id);
create index idx_ventes_session on public.ventes(session_id);
create index idx_ventes_source on public.ventes(source_vente_id);
create index idx_ventes_statut_dtype on public.ventes(statut, dtype, date);
create index idx_ventes_magasin on public.ventes(magasin_id, date);
create index idx_vente_articles_tenant on public.vente_articles(tenant_id);
create index idx_vente_articles_vente on public.vente_articles(vente_id);
create index idx_vente_articles_article on public.vente_articles(article_id);
create index idx_vente_paiements_tenant on public.vente_paiements(tenant_id);
create index idx_vente_paiements_vente on public.vente_paiements(vente_id);
create index idx_vente_paiements_session on public.vente_paiements(session_id, mode);
create index idx_vente_lots_tenant on public.vente_lots(tenant_id);
create index idx_vente_lots_vente on public.vente_lots(vente_id);
create index idx_journal_caisse_tenant on public.journal_caisse(tenant_id);
create index idx_journal_date on public.journal_caisse(tenant_id, date);
create index idx_journal_session on public.journal_caisse(session_id);
create index idx_cheques_tenant on public.cheques(tenant_id);
create index idx_mouvements_stock_tenant on public.mouvements_stock(tenant_id);
create index idx_mouvements_article on public.mouvements_stock(article_id);
create index idx_mouvements_magasin on public.mouvements_stock(magasin_id, date);
create index idx_mouvements_fidelite_tenant on public.mouvements_fidelite(tenant_id);
create index idx_fidelite_client on public.mouvements_fidelite(client_id, date);
