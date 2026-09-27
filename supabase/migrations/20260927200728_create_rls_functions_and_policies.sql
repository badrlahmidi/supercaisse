create or replace function public.is_super_admin()
returns boolean
language sql
security definer
stable
set search_path = public
as $$
  select coalesce((select is_super_admin from public.profiles where id = auth.uid()), false)
$$;

create or replace function public.current_tenant_id()
returns uuid
language sql
security definer
stable
set search_path = public
as $$
  select tenant_id from public.profiles where id = auth.uid()
$$;

alter table public.tenants enable row level security;

create policy tenants_select on public.tenants
  for select
  using (
    public.is_super_admin()
    or id = public.current_tenant_id()
  );

create policy tenants_write on public.tenants
  for all
  using (public.is_super_admin())
  with check (public.is_super_admin());

alter table public.profiles enable row level security;

create policy profiles_select on public.profiles
  for select
  using (
    id = auth.uid()
    or public.is_super_admin()
    or (
      tenant_id = public.current_tenant_id()
      and exists (
        select 1 from public.profiles p
        where p.id = auth.uid() and p.role = 'admin_tenant'
      )
    )
  );

alter table public.magasins enable row level security;
create policy tenant_isolation on public.magasins
  using (public.is_super_admin() or tenant_id = public.current_tenant_id());

alter table public.articles_cache enable row level security;
create policy tenant_isolation on public.articles_cache
  using (public.is_super_admin() or tenant_id = public.current_tenant_id());

alter table public.commande_terrain enable row level security;
create policy tenant_isolation on public.commande_terrain
  using (public.is_super_admin() or tenant_id = public.current_tenant_id());

alter table public.commande_terrain_ligne enable row level security;
create policy tenant_isolation on public.commande_terrain_ligne
  using (
    exists (
      select 1 from public.commande_terrain c
      where c.id = commande_id
        and (public.is_super_admin() or c.tenant_id = public.current_tenant_id())
    )
  );

alter table public.commande_terrain_historique enable row level security;
create policy tenant_isolation on public.commande_terrain_historique
  using (
    exists (
      select 1 from public.commande_terrain c
      where c.id = commande_id
        and (public.is_super_admin() or c.tenant_id = public.current_tenant_id())
    )
  );

alter table public.evenements_business enable row level security;
create policy tenant_isolation on public.evenements_business
  using (public.is_super_admin() or tenant_id = public.current_tenant_id());
