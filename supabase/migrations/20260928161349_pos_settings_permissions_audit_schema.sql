create or replace function public.can_manage_settings()
returns boolean
language sql
security definer
stable
set search_path = public
as $$
  select coalesce(
    (select is_super_admin or role = 'admin_tenant' or pos_role = 'admin'
     from public.profiles where id = auth.uid() and actif = true),
    false
  )
$$;

revoke execute on function public.can_manage_settings() from public, anon;
grant execute on function public.can_manage_settings() to authenticated;

-- tables_resto (dine-in table status, not gated by the module permission grid)
create table public.tables_resto (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  nom text not null,
  statut text not null default 'libre',
  ticket_id text
);

-- audit_log (append-only; writing is automatic for any active member, viewing is admin-only)
create table public.audit_log (
  id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references public.tenants(id),
  date timestamptz not null default now(),
  utilisateur_id uuid references public.profiles(id),
  action text not null,
  detail text,
  reference_type text,
  reference_id uuid
);

-- settings (key/value per tenant)
create table public.settings (
  tenant_id uuid not null references public.tenants(id),
  key text not null,
  value text not null,
  primary key (tenant_id, key)
);

-- permissions: per-tenant customizable pos_role x module x action grid.
-- Not yet wired into the RLS policies created in earlier migrations (those
-- still use the hardcoded can_manage_pos()/has_pos_role() defaults) — this
-- is storage for the future dynamic permission model, seeded with the same
-- defaults the local SQLite app seeds on a fresh install.
create table public.permissions (
  tenant_id uuid not null references public.tenants(id),
  pos_role text not null check (pos_role in ('admin','manager','caissier')),
  module text not null,
  action text not null,
  allowed boolean not null default true,
  primary key (tenant_id, pos_role, module, action)
);

-- tenant-consistency triggers
create or replace function public.check_settings_tenant_consistency()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  if TG_TABLE_NAME = 'audit_log' and new.utilisateur_id is not null then
    if not exists (select 1 from public.profiles p where p.id = new.utilisateur_id and p.tenant_id = new.tenant_id) then
      raise exception 'utilisateur_id n''appartient pas à ce tenant';
    end if;
  end if;
  return new;
end;
$$;

revoke execute on function public.check_settings_tenant_consistency() from public, anon, authenticated;

create trigger check_audit_log_tenant_consistency before insert or update on public.audit_log for each row execute function public.check_settings_tenant_consistency();

-- RLS
alter table public.tables_resto enable row level security;
alter table public.audit_log enable row level security;
alter table public.settings enable row level security;
alter table public.permissions enable row level security;

create policy tables_resto_select on public.tables_resto for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy tables_resto_write on public.tables_resto for all
  using (
    public.is_super_admin()
    or (tenant_id = public.current_tenant_id() and (public.has_role('admin_tenant') or public.has_pos_role('admin') or public.has_pos_role('manager') or public.has_pos_role('caissier')))
  )
  with check (
    public.is_super_admin()
    or (tenant_id = public.current_tenant_id() and (public.has_role('admin_tenant') or public.has_pos_role('admin') or public.has_pos_role('manager') or public.has_pos_role('caissier')))
  );

create policy audit_log_select on public.audit_log for select
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_settings()));
create policy audit_log_insert on public.audit_log for insert
  with check (
    public.is_super_admin()
    or (tenant_id = public.current_tenant_id() and (public.has_role('admin_tenant') or public.has_pos_role('admin') or public.has_pos_role('manager') or public.has_pos_role('caissier')))
  );

create policy settings_select on public.settings for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy settings_write on public.settings for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_settings()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_settings()));

create policy permissions_select on public.permissions for select using (public.is_super_admin() or tenant_id = public.current_tenant_id());
create policy permissions_write on public.permissions for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_settings()))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.can_manage_settings()));

-- indexes
create index idx_tables_resto_tenant on public.tables_resto(tenant_id);
create index idx_audit_log_tenant on public.audit_log(tenant_id);
create index idx_audit_log_date on public.audit_log(tenant_id, date);
create index idx_audit_log_action on public.audit_log(tenant_id, action);
create index idx_settings_tenant on public.settings(tenant_id);
create index idx_permissions_tenant on public.permissions(tenant_id);

-- seed the default permission grid for existing tenants, matching what the
-- local SQLite app seeds on a fresh install (migration_001_base in db.rs)
do $$
declare
  t record;
  modules text[] := array['articles','categories','clients','fournisseurs','ventes','achats','stock','inventaire','journal','cheques','rapports','magasins','audit','settings','reappro'];
  actions text[] := array['voir','creer','modifier','exporter'];
  manager_denied text[] := array['magasins','audit','settings'];
  m text;
  a text;
  caissier_ok boolean;
begin
  for t in select id from public.tenants loop
    foreach m in array modules loop
      foreach a in array actions loop
        insert into public.permissions (tenant_id, pos_role, module, action, allowed)
        values (t.id, 'admin', m, a, true)
        on conflict do nothing;

        insert into public.permissions (tenant_id, pos_role, module, action, allowed)
        values (t.id, 'manager', m, a, not (m = any(manager_denied)))
        on conflict do nothing;

        caissier_ok := (m = 'ventes' and a in ('voir', 'creer')) or (m = 'clients' and a = 'voir');
        insert into public.permissions (tenant_id, pos_role, module, action, allowed)
        values (t.id, 'caissier', m, a, caissier_ok)
        on conflict do nothing;
      end loop;
    end loop;
  end loop;
end;
$$;
