create or replace function public.has_role(target_role text)
returns boolean
language sql
security definer
stable
set search_path = public
as $$
  select coalesce((select role = target_role from public.profiles where id = auth.uid() and actif = true), false)
$$;

revoke execute on function public.has_role(text) from public, anon;
grant execute on function public.has_role(text) to authenticated;

-- magasins
drop policy tenant_isolation on public.magasins;

create policy magasins_select on public.magasins
  for select
  using (public.is_super_admin() or tenant_id = public.current_tenant_id());

create policy magasins_write on public.magasins
  for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.has_role('admin_tenant')))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.has_role('admin_tenant')));

-- articles_cache
drop policy tenant_isolation on public.articles_cache;

create policy articles_cache_select on public.articles_cache
  for select
  using (public.is_super_admin() or tenant_id = public.current_tenant_id());

create policy articles_cache_write on public.articles_cache
  for all
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.has_role('admin_tenant')))
  with check (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.has_role('admin_tenant')));

-- commande_terrain
drop policy tenant_isolation on public.commande_terrain;

create policy commande_terrain_select on public.commande_terrain
  for select
  using (public.is_super_admin() or tenant_id = public.current_tenant_id());

create policy commande_terrain_insert on public.commande_terrain
  for insert
  with check (
    public.is_super_admin()
    or (
      tenant_id = public.current_tenant_id()
      and (
        public.has_role('admin_tenant')
        or (public.has_role('commercial') and commercial_id = auth.uid())
      )
    )
  );

create policy commande_terrain_update on public.commande_terrain
  for update
  using (
    public.is_super_admin()
    or (
      tenant_id = public.current_tenant_id()
      and (
        public.has_role('admin_tenant')
        or public.has_role('receptionniste')
        or (public.has_role('commercial') and commercial_id = auth.uid())
      )
    )
  )
  with check (
    public.is_super_admin()
    or (
      tenant_id = public.current_tenant_id()
      and (
        public.has_role('admin_tenant')
        or public.has_role('receptionniste')
        or (public.has_role('commercial') and commercial_id = auth.uid())
      )
    )
  );

create policy commande_terrain_delete on public.commande_terrain
  for delete
  using (public.is_super_admin() or (tenant_id = public.current_tenant_id() and public.has_role('admin_tenant')));

-- commande_terrain_ligne
drop policy tenant_isolation on public.commande_terrain_ligne;

create policy commande_terrain_ligne_select on public.commande_terrain_ligne
  for select
  using (
    exists (
      select 1 from public.commande_terrain c
      where c.id = commande_id
        and (public.is_super_admin() or c.tenant_id = public.current_tenant_id())
    )
  );

create policy commande_terrain_ligne_write on public.commande_terrain_ligne
  for all
  using (
    exists (
      select 1 from public.commande_terrain c
      where c.id = commande_id
        and (
          public.is_super_admin()
          or (
            c.tenant_id = public.current_tenant_id()
            and (
              public.has_role('admin_tenant')
              or (public.has_role('commercial') and c.commercial_id = auth.uid())
            )
          )
        )
    )
  )
  with check (
    exists (
      select 1 from public.commande_terrain c
      where c.id = commande_id
        and (
          public.is_super_admin()
          or (
            c.tenant_id = public.current_tenant_id()
            and (
              public.has_role('admin_tenant')
              or (public.has_role('commercial') and c.commercial_id = auth.uid())
            )
          )
        )
    )
  );

-- commande_terrain_historique (append-only)
drop policy tenant_isolation on public.commande_terrain_historique;

create policy commande_terrain_historique_select on public.commande_terrain_historique
  for select
  using (
    exists (
      select 1 from public.commande_terrain c
      where c.id = commande_id
        and (public.is_super_admin() or c.tenant_id = public.current_tenant_id())
    )
  );

create policy commande_terrain_historique_insert on public.commande_terrain_historique
  for insert
  with check (
    exists (
      select 1 from public.commande_terrain c
      where c.id = commande_id
        and (
          public.is_super_admin()
          or (
            c.tenant_id = public.current_tenant_id()
            and (
              public.has_role('admin_tenant')
              or public.has_role('receptionniste')
              or (public.has_role('commercial') and c.commercial_id = auth.uid())
            )
          )
        )
    )
  );

-- evenements_business (append-only)
drop policy tenant_isolation on public.evenements_business;

create policy evenements_business_select on public.evenements_business
  for select
  using (public.is_super_admin() or tenant_id = public.current_tenant_id());

create policy evenements_business_insert on public.evenements_business
  for insert
  with check (
    public.is_super_admin()
    or (
      tenant_id = public.current_tenant_id()
      and (public.has_role('admin_tenant') or public.has_role('commercial') or public.has_role('receptionniste'))
    )
  );
