create policy profiles_insert on public.profiles
  for insert
  with check (
    public.is_super_admin()
    or (
      is_super_admin = false
      and tenant_id = public.current_tenant_id()
      and exists (
        select 1 from public.profiles p
        where p.id = auth.uid() and p.role = 'admin_tenant'
      )
    )
  );

create policy profiles_update on public.profiles
  for update
  using (
    public.is_super_admin()
    or (
      tenant_id = public.current_tenant_id()
      and exists (
        select 1 from public.profiles p
        where p.id = auth.uid() and p.role = 'admin_tenant'
      )
    )
  )
  with check (
    public.is_super_admin()
    or (
      is_super_admin = false
      and tenant_id = public.current_tenant_id()
      and exists (
        select 1 from public.profiles p
        where p.id = auth.uid() and p.role = 'admin_tenant'
      )
    )
  );
